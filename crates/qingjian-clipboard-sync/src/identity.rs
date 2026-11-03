//! 这台设备的长期身份：一对 X25519 密钥，私钥只留在本机。

use std::path::Path;

use crate::channel::noise_params;
use crate::error::SyncError;
use crate::peers::{hex, write_private};

#[derive(Clone)]
pub struct Identity {
    pub(crate) private: [u8; 32],

    pub public: [u8; 32],
}

impl Identity {
    pub fn generate() -> Result<Self, SyncError> {
        let keypair = snow::Builder::new(noise_params(false)).generate_keypair()?;
        Self::from_bytes(&keypair.private, &keypair.public)
    }

    /// 读保存的身份，没有就生成一个存下。文件坏了报错、不覆盖：覆盖了所有配对都会失效。
    pub fn load_or_create(path: &Path) -> Result<Self, SyncError> {
        let storage = |error: std::io::Error| SyncError::Storage(error.to_string());
        match std::fs::read(path) {
            Ok(bytes) if bytes.len() == 64 => Self::from_bytes(&bytes[..32], &bytes[32..]),
            Ok(_) => Err(SyncError::Storage("identity file is corrupt".to_owned())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let identity = Self::generate()?;
                let bytes = [identity.private, identity.public].concat();
                write_private(path, &bytes).map_err(storage)?;
                Ok(identity)
            }
            Err(error) => Err(storage(error)),
        }
    }

    fn from_bytes(private: &[u8], public: &[u8]) -> Result<Self, SyncError> {
        let corrupt = |_| SyncError::Storage("identity key has a wrong length".to_owned());
        Ok(Self {
            private: private.try_into().map_err(corrupt)?,
            public: public.try_into().map_err(corrupt)?,
        })
    }

    /// 作为名单键的设备 id。
    pub fn device_id(&self) -> String {
        hex(&self.public)
    }
}
