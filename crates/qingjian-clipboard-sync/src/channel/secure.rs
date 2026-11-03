//! [`SecureChannel`]：握手完成后的加密消息通道。一条消息先发 4 字节长度，再分块发；每块都单独加密。

use std::io::{Read, Write};

use snow::TransportState;

use super::noise_params;
use crate::error::SyncError;
use crate::identity::Identity;

/// Noise 单条消息的上限。
const MAX_NOISE_MESSAGE: usize = 65535;

const TAG_LENGTH: usize = 16;

const MAX_CHUNK: usize = MAX_NOISE_MESSAGE - TAG_LENGTH;

/// 一条消息的上限：比最大的图片（32 MB）留些余量，防对方报一个巨大的长度把内存撑爆。
pub(crate) const MAX_MESSAGE: usize = 48 * 1024 * 1024;

pub struct SecureChannel<S> {
    stream: S,

    transport: TransportState,

    remote: [u8; 32],
}

impl<S: Read + Write> SecureChannel<S> {
    pub(super) fn handshake(
        mut stream: S,
        identity: &Identity,
        psk: Option<&[u8; 32]>,
        initiator: bool,
    ) -> Result<Self, SyncError> {
        let mut builder =
            snow::Builder::new(noise_params(psk.is_some())).local_private_key(&identity.private)?;
        if let Some(psk) = psk {
            builder = builder.psk(3, psk)?;
        }
        let mut state = if initiator {
            builder.build_initiator()?
        } else {
            builder.build_responder()?
        };
        let mut buffer = vec![0_u8; MAX_NOISE_MESSAGE];
        let mut sending = initiator;
        while !state.is_handshake_finished() {
            if sending {
                let length = state.write_message(&[], &mut buffer)?;
                write_frame(&mut stream, &buffer[..length])?;
            } else {
                let frame = read_frame(&mut stream)?;
                state.read_message(&frame, &mut buffer)?;
            }
            sending = !sending;
        }
        let remote = state
            .get_remote_static()
            .and_then(|key| key.try_into().ok())
            .ok_or(SyncError::Handshake)?;
        Ok(Self {
            stream,
            transport: state.into_transport_mode()?,
            remote,
        })
    }

    /// 对方的长期公钥，握手已证明对方持有它。
    pub fn remote(&self) -> &[u8; 32] {
        &self.remote
    }

    pub fn send(&mut self, message: &[u8]) -> Result<(), SyncError> {
        if message.len() > MAX_MESSAGE {
            return Err(SyncError::Protocol("message too large".to_owned()));
        }
        let length = u32::try_from(message.len())
            .map_err(|_| SyncError::Protocol("message too large".to_owned()))?;
        let mut buffer = vec![0_u8; MAX_NOISE_MESSAGE];
        self.send_chunk(&length.to_be_bytes(), &mut buffer)?;
        for chunk in message.chunks(MAX_CHUNK) {
            self.send_chunk(chunk, &mut buffer)?;
        }
        self.stream.flush()?;
        Ok(())
    }

    fn send_chunk(&mut self, chunk: &[u8], buffer: &mut [u8]) -> Result<(), SyncError> {
        let length = self.transport.write_message(chunk, buffer)?;
        write_frame(&mut self.stream, &buffer[..length])
    }

    pub fn receive(&mut self) -> Result<Vec<u8>, SyncError> {
        self.receive_with(&mut |_, _| {})
    }

    /// 同 [`Self::receive`]，每收一块报一次（已收，总共）。
    pub fn receive_with(
        &mut self,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<Vec<u8>, SyncError> {
        let mut buffer = vec![0_u8; MAX_NOISE_MESSAGE];
        let header = self.receive_chunk(&mut buffer)?;
        let header: [u8; 4] = buffer[..header]
            .try_into()
            .map_err(|_| SyncError::Protocol("bad message header".to_owned()))?;
        let total = u32::from_be_bytes(header) as usize;
        if total > MAX_MESSAGE {
            return Err(SyncError::Protocol("message too large".to_owned()));
        }
        // 不按对方报的长度预留：内容到了才长
        let mut message = Vec::new();
        while message.len() < total {
            let length = self.receive_chunk(&mut buffer)?;
            if length == 0 || message.len() + length > total {
                return Err(SyncError::Protocol("bad message chunk".to_owned()));
            }
            message.extend_from_slice(&buffer[..length]);
            progress(message.len() as u64, total as u64);
        }
        Ok(message)
    }

    fn receive_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, SyncError> {
        let frame = read_frame(&mut self.stream)?;
        Ok(self.transport.read_message(&frame, buffer)?)
    }
}

fn write_frame<S: Write>(stream: &mut S, frame: &[u8]) -> Result<(), SyncError> {
    let length = u16::try_from(frame.len()).map_err(|_| SyncError::Handshake)?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(frame)?;
    stream.flush()?;
    Ok(())
}

fn read_frame<S: Read>(stream: &mut S) -> Result<Vec<u8>, SyncError> {
    let mut length = [0_u8; 2];
    stream.read_exact(&mut length)?;
    let mut frame = vec![0_u8; usize::from(u16::from_be_bytes(length))];
    stream.read_exact(&mut frame)?;
    Ok(frame)
}
