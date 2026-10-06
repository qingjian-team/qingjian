use std::path::PathBuf;

use qingjian_dictionary::DictionaryError;
use qingjian_learning::LearningError;
use qingjian_lm::LmError;
use qingjian_platform::ConfigError;
use qingjian_translate::GlossaryError;

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    /// `.app` 里找不到 Resources 目录，说明不是通过 bundle.sh 打包的。
    #[error("bundle has no resource directory")]
    NoResources,

    /// 词库文件缺失。
    #[error("resource not found: {0}")]
    MissingResource(PathBuf),

    #[error(transparent)]
    Dictionary(#[from] DictionaryError),

    #[error(transparent)]
    Glossary(#[from] GlossaryError),

    #[error(transparent)]
    Learning(#[from] LearningError),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    LanguageModel(#[from] LmError),

    /// emoji 表读取或解析失败。
    #[error("emoji table: {0}")]
    Emoji(#[from] std::io::Error),
}
