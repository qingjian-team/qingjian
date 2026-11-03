//! 加密通道：连接开头一个字节说明是配对还是通信，之后走 Noise。配对多一步 SPAKE2，换出的密钥作 Noise 的 PSK。
//! 6 位码不能只靠两边比对：中间人可以穷举自己的临时密钥凑出相同的码，所以用 PAKE，每次连接只有一次猜测机会。

#[cfg(test)]
mod tests;

mod secure;

pub use secure::SecureChannel;

use std::io::{Read, Write};

use snow::params::NoiseParams;
use spake2::{Ed25519Group, Identity as SpakeIdentity, Password, Spake2};

use crate::error::SyncError;
use crate::identity::Identity;

const MODE_SYNC: u8 = 1;
const MODE_PAIR: u8 = 2;

/// 问对方是谁（扫描用），不加密、不要配对，回答只有 id、名字和型号。
pub(crate) const MODE_INFO: u8 = 3;

const SPAKE_CONTEXT: &[u8] = b"qingjian-clipboard-pair-v1";

const PAIR_CONFIRM: &[u8] = b"paired";

/// 接受方已经不认这台设备时回的一句，发起方据此也把对方删掉。走的是认证过的通道，别人冒充不了；
/// 同步的消息都是 JSON 或以记录头开头，不会是这几个字节。
pub(crate) const NOT_PAIRED: &[u8] = b"\0not-paired";

/// SPAKE2 消息的长度上限，正常是 33 字节。
const MAX_SPAKE_MESSAGE: usize = 256;

/// 连接开头读到的是哪一种。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opening {
    Sync,
    Pair,
    Info,
}

pub(crate) fn noise_params(with_psk: bool) -> NoiseParams {
    let pattern = if with_psk {
        "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s"
    } else {
        "Noise_XX_25519_ChaChaPoly_BLAKE2s"
    };
    pattern.parse().expect("the pattern is a valid constant")
}

/// 生成 6 位配对码，用一次就作废由调用方保证。
pub fn generate_pair_code() -> Result<String, SyncError> {
    let random = snow::Builder::new(noise_params(false)).generate_keypair()?;
    let value = random
        .private
        .iter()
        .take(8)
        .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte));
    Ok(format!("{:06}", value % 1_000_000))
}

/// 主动连已配对的设备。`known` 判断对方公钥在不在名单里。
pub fn connect<S: Read + Write>(
    mut stream: S,
    identity: &Identity,
    known: impl Fn(&[u8; 32]) -> bool,
) -> Result<SecureChannel<S>, SyncError> {
    stream.write_all(&[MODE_SYNC])?;
    let channel = SecureChannel::handshake(stream, identity, None, true)?;
    if known(channel.remote()) {
        Ok(channel)
    } else {
        Err(SyncError::UnknownPeer)
    }
}

/// 读连接开头那个字节，再决定调 [`accept`] 还是 [`pair_host`]。
pub fn read_opening<S: Read>(stream: &mut S) -> Result<Opening, SyncError> {
    let mut mode = [0_u8; 1];
    stream.read_exact(&mut mode)?;
    match mode[0] {
        MODE_SYNC => Ok(Opening::Sync),
        MODE_PAIR => Ok(Opening::Pair),
        MODE_INFO => Ok(Opening::Info),
        other => Err(SyncError::Protocol(format!("unknown mode {other}"))),
    }
}

/// 接受已配对设备的连接（[`read_opening`] 之后）。
pub fn accept<S: Read + Write>(
    stream: S,
    identity: &Identity,
    known: impl Fn(&[u8; 32]) -> bool,
) -> Result<SecureChannel<S>, SyncError> {
    let mut channel = SecureChannel::handshake(stream, identity, None, false)?;
    if known(channel.remote()) {
        return Ok(channel);
    }
    // 告诉对方这边已经取消配对（取消时它可能不在线）。再读它的第一条才断：
    // 直接断开时对方的数据还在路上，系统会重置连接，对方可能读不到这句
    if channel.send(NOT_PAIRED).is_ok() {
        let _ = channel.receive();
    }
    Err(SyncError::UnknownPeer)
}

/// 显示配对码的一方（[`read_opening`] 之后）。无论成败这个码都要作废。
pub fn pair_host<S: Read + Write>(
    mut stream: S,
    identity: &Identity,
    code: &str,
) -> Result<SecureChannel<S>, SyncError> {
    let psk = exchange_spake(&mut stream, code)?;
    let mut channel = SecureChannel::handshake(stream, identity, Some(&psk), false)?;
    channel.send(PAIR_CONFIRM)?;
    Ok(channel)
}

/// 输入配对码的一方。
pub fn pair_join<S: Read + Write>(
    mut stream: S,
    identity: &Identity,
    code: &str,
) -> Result<SecureChannel<S>, SyncError> {
    stream.write_all(&[MODE_PAIR])?;
    let psk = exchange_spake(&mut stream, code)?;
    let mut channel = SecureChannel::handshake(stream, identity, Some(&psk), true)?;
    // PSK 在最后一条握手消息才生效，发起方写完就以为成功了；码对不对要等对方解开后回这句才知道
    if channel.receive().map_err(|_| SyncError::Handshake)? != PAIR_CONFIRM {
        return Err(SyncError::Handshake);
    }
    Ok(channel)
}

fn exchange_spake<S: Read + Write>(stream: &mut S, code: &str) -> Result<[u8; 32], SyncError> {
    let (state, outbound) = Spake2::<Ed25519Group>::start_symmetric(
        &Password::new(code.as_bytes()),
        &SpakeIdentity::new(SPAKE_CONTEXT),
    );
    let length = u16::try_from(outbound.len()).map_err(|_| SyncError::Handshake)?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(&outbound)?;
    stream.flush()?;

    let mut length = [0_u8; 2];
    stream.read_exact(&mut length)?;
    let length = usize::from(u16::from_be_bytes(length));
    if length > MAX_SPAKE_MESSAGE {
        return Err(SyncError::Handshake);
    }
    let mut inbound = vec![0_u8; length];
    stream.read_exact(&mut inbound)?;
    let key = state.finish(&inbound).map_err(|_| SyncError::Handshake)?;
    key.as_slice().try_into().map_err(|_| SyncError::Handshake)
}
