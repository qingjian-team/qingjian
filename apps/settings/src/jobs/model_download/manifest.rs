use serde::Deserialize;

/// Release 里的 `manifest.json`（模型仓库 `pack.sh` 写的）：每块与拼起来的整个压缩包的大小和 SHA-256。
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub tag: String,
    pub archive: Piece,
    pub parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
pub struct Piece {
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
pub struct Part {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
}
