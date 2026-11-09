use qingjian_dictionary::DictionaryError;
use qingjian_learning::LearningError;
use qingjian_lm::LmError;
use qingjian_neural::NeuralError;
use qingjian_platform::ConfigError;
use qingjian_predict::PredictError;
use qingjian_translate::GlossaryError;
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(transparent)]
    Cold(#[from] crate::cold::error::ColdError),

    #[error(transparent)]
    Dictionary(#[from] DictionaryError),

    #[error(transparent)]
    Neural(#[from] NeuralError),

    #[error(transparent)]
    Glossary(#[from] GlossaryError),

    #[error(transparent)]
    Learning(#[from] LearningError),

    /// 学习语言不是 en / ja / es / de。
    #[error("learning language must be en, ja, es or de, got {0:?}")]
    Language(String),

    /// 删用户词后读回核实：要删的词仍在 `user-words.tsv` 里，说明没写进磁盘。
    #[error("用户词删除未落盘：{0} 仍在 user-words.tsv 里")]
    UserWordsRemain(String),

    #[error(transparent)]
    Config(#[from] ConfigError),

    /// `--code` 给的目录不是能用的形码方案（id 不合法、`scheme.toml` 坏了或码表不在，详情在日志里）。
    #[error("{0} is not a usable code scheme directory")]
    CodeScheme(std::path::PathBuf),

    /// `--shuangpin` 给的目录不是能用的双拼方案（没写 `type = "shuangpin"`、键位写错了，详情在日志里）。
    #[error("{0} is not a usable shuangpin scheme directory")]
    ShuangpinScheme(std::path::PathBuf),

    /// `--aux-table` 给的目录不是能用的辅码方案（没写 `type = "aux"`、码表不在或词码规则写错了，详情在日志里）。
    #[error("{0} is not a usable aux code scheme directory")]
    AuxScheme(std::path::PathBuf),

    #[error(transparent)]
    Predict(#[from] PredictError),

    #[error(transparent)]
    LanguageModel(#[from] LmError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Replay(#[from] crate::replay::ReplayError),

    #[error(transparent)]
    Eval(#[from] crate::eval::EvalError),

    #[error(transparent)]
    Tune(#[from] crate::tuning::TuneError),
}
