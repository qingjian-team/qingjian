use std::path::{Path, PathBuf};

use qingjian_lm::{BigramModel, LmError};

/// 语言模型的数据文件：打包过的 `lm.qj` 优先，没有就用两张 TSV。
pub enum LanguageModelFiles {
    Packed(PathBuf),

    Tsv { unigram: PathBuf, bigram: PathBuf },
}

impl LanguageModelFiles {
    /// 在 `dir` 里找：`lm.qj`，否则 `lm-unigram.tsv` + `lm-bigram.tsv`；都没有为 `None`。
    pub fn find(dir: &Path) -> Option<Self> {
        let packed = dir.join("lm.qj");
        if packed.is_file() {
            return Some(Self::Packed(packed));
        }
        let unigram = dir.join("lm-unigram.tsv");
        let bigram = dir.join("lm-bigram.tsv");
        (unigram.is_file() && bigram.is_file()).then_some(Self::Tsv { unigram, bigram })
    }

    pub(super) fn load(&self) -> Result<BigramModel, LmError> {
        match self {
            Self::Packed(path) => BigramModel::from_path(path),
            Self::Tsv { unigram, bigram } => BigramModel::from_paths(unigram, bigram),
        }
    }
}
