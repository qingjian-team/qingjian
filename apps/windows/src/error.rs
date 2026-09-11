use qingjian_dictionary::DictionaryError;
use qingjian_translate::GlossaryError;

/// 装配 Engine 时的错误。
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("load dictionary: {0}")]
    Dictionary(#[from] DictionaryError),

    #[error("load glossary: {0}")]
    Glossary(#[from] GlossaryError),
}
