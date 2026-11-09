//! 加密连接：rustls（ring + 内置 Mozilla 根证书），桌面与安卓同一套，不碰平台证书库。

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use crate::VoiceError;

pub type TlsStream = StreamOwned<ClientConnection, TcpStream>;

/// 连 `host:port` 并握手前的 TCP 部分；TLS 握手随第一次读写进行（tungstenite 握手时）。
pub fn connect(host: &str, port: u16, timeout: Duration) -> Result<TlsStream, VoiceError> {
    let address = (host, port)
        .to_socket_addrs()
        .map_err(|error| VoiceError::Connect(error.to_string()))?
        .next()
        .ok_or_else(|| VoiceError::Connect(format!("{host} 解析不出地址")))?;
    let tcp = TcpStream::connect_timeout(&address, timeout)
        .map_err(|error| VoiceError::Connect(error.to_string()))?;
    tcp.set_nodelay(true)
        .and_then(|()| tcp.set_read_timeout(Some(timeout)))
        .and_then(|()| tcp.set_write_timeout(Some(timeout)))
        .map_err(|error| VoiceError::Connect(error.to_string()))?;
    let name = ServerName::try_from(host.to_owned())
        .map_err(|error| VoiceError::Connect(error.to_string()))?;
    let connection = ClientConnection::new(config()?, name)
        .map_err(|error| VoiceError::Connect(error.to_string()))?;
    Ok(StreamOwned::new(connection, tcp))
}

/// 进程里共用一份客户端配置（根证书表建一次）。
fn config() -> Result<Arc<ClientConfig>, VoiceError> {
    static CONFIG: OnceLock<Result<Arc<ClientConfig>, String>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .map(|builder| {
                    Arc::new(builder.with_root_certificates(roots).with_no_client_auth())
                })
                .map_err(|error| error.to_string())
        })
        .clone()
        .map_err(VoiceError::Connect)
}
