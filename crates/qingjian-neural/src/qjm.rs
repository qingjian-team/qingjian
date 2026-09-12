//! `.qjm`：本地整句模型的单文件形态——就是 `.qj` 容器（`qingjian-format`）装了 [`Kind::Model`]：
//! `META`（名称 / 许可证 / 署名，条数记参数量）+ `CONF`（`config.json` 原文）+ `VOCB`（`vocab.json` 原文）
//! + `SAFT`（`model.safetensors` 原文，mmap 后切片直接给 candle）。
//!
//! 训练仓库导出的仍是三件套目录，开发时直接加载；随包与用户目录用 `.qjm`，`dict-convert pack model` 把前者打成后者。
//! safetensors 在这里是不透明载荷，容器版本只管自己那一层。

use std::path::{Path, PathBuf};

use candle_core::safetensors::SliceSafetensors;
use qingjian_format::{Kind, Metadata, Writer};

use crate::{ModelConfig, NeuralError, Vocab};

/// 单文件模型的扩展名。
pub const EXTENSION: &str = "qjm";

/// 三件套目录里的权重文件。
pub const WEIGHTS_FILE: &str = "model.safetensors";

/// 三件套目录里的结构配置。
pub const CONFIG_FILE: &str = "config.json";

/// 三件套目录里的字表。
pub const VOCAB_FILE: &str = "vocab.json";

/// `config.json` 原文那一节。
pub const CONFIG_TAG: [u8; 4] = *b"CONF";

/// `vocab.json` 原文那一节。
pub const VOCAB_TAG: [u8; 4] = *b"VOCB";

/// `model.safetensors` 原文那一节。
pub const WEIGHTS_TAG: [u8; 4] = *b"SAFT";

/// `dir` 下能加载的模型：先找 `.qjm` 单文件（有几份按文件名取第一份），没有再看三件套（返回目录本身）；都没有为 `None`。
pub fn find_model(dir: &Path) -> Option<PathBuf> {
    let mut packed: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == EXTENSION) && path.is_file())
        .collect();
    packed.sort();
    if let Some(first) = packed.into_iter().next() {
        return Some(first);
    }
    dir.join(WEIGHTS_FILE).is_file().then(|| dir.to_path_buf())
}

/// 把三件套目录打成一个 `.qjm`：三个文件原文各占一节，写之前把结构、字表、权重都解析一遍，坏文件在这里就报出来。
/// `metadata.entries` 是 0 就填成参数量。返回参数量。
pub fn pack(dir: &Path, out: &Path, metadata: &Metadata) -> Result<u64, NeuralError> {
    let config = read(&dir.join(CONFIG_FILE))?;
    let vocab = read(&dir.join(VOCAB_FILE))?;
    let weights = read(&dir.join(WEIGHTS_FILE))?;
    let cfg = ModelConfig::from_json(&String::from_utf8_lossy(&config), &dir.join(CONFIG_FILE))?;
    let table = Vocab::from_json(&String::from_utf8_lossy(&vocab), &dir.join(VOCAB_FILE))?;
    if table.len() != cfg.vocab_size {
        return Err(NeuralError::Corrupt("vocab.json size differs from config"));
    }
    let parameters: u64 = SliceSafetensors::new(&weights)?
        .tensors()
        .iter()
        .map(|(_, view)| view.shape().iter().map(|&n| n as u64).product::<u64>())
        .sum();
    let metadata = Metadata {
        entries: if metadata.entries == 0 {
            parameters
        } else {
            metadata.entries
        },
        ..metadata.clone()
    };
    Writer::new(Kind::Model, &metadata)?
        .section(CONFIG_TAG, &config)
        .section(VOCAB_TAG, &vocab)
        .section(WEIGHTS_TAG, &weights)
        .write_to(out)?;
    Ok(parameters)
}

fn read(path: &Path) -> Result<Vec<u8>, NeuralError> {
    std::fs::read(path).map_err(|source| NeuralError::Io {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("qingjian-neural-tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_packed_file_before_loose_files() {
        let dir = scratch("find");
        assert_eq!(find_model(&dir), None);
        std::fs::write(dir.join(WEIGHTS_FILE), b"").unwrap();
        assert_eq!(find_model(&dir).as_deref(), Some(dir.as_path()));
        std::fs::write(dir.join("b.qjm"), b"").unwrap();
        std::fs::write(dir.join("a.qjm"), b"").unwrap();
        assert_eq!(find_model(&dir), Some(dir.join("a.qjm")));
        // 同名目录不算文件
        std::fs::create_dir(dir.join("0.qjm")).unwrap();
        assert_eq!(find_model(&dir), Some(dir.join("a.qjm")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pack_rejects_missing_files() {
        let dir = scratch("pack-missing");
        let error = pack(&dir, &dir.join("out.qjm"), &Metadata::default()).unwrap_err();
        assert!(matches!(error, NeuralError::Io { .. }), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
