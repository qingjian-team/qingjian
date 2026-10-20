//! 本机模型实时翻译：维护句子预览，以可取消的后台请求更新整句译文。

mod config;
mod sentence;
mod translation;
mod translator;

pub use config::LiveTranslationConfig;
pub use sentence::SentenceTracker;
pub use translation::Translation;
pub use translator::LiveTranslator;
