//! 装配 Engine：Server 进程里唯一知道具体 Translator / Learner 类型的地方（对应 macOS 的 `host::init`）。
//!
//! 现在只接词库 + 释义表 + 词频学习，够跑通「拼音 → 候选 → 上屏」。语言模型、emoji、英文词表、
//! 云联想、本地整句模型随后补（与 CLI 的 `build_engine` 对齐）。

use std::path::Path;

use qingjian_core::{Engine, Language};
use qingjian_dictionary::Dictionary;
use qingjian_learning::FrequencyLearner;
use qingjian_translate::Glossary;

use crate::error::ServerError;

/// 从词库文件装配一个 Engine，可选接一门学习语言的释义表。
pub fn assemble(dict: &Path, glossary: Option<(Language, &Path)>) -> Result<Engine, ServerError> {
    let dictionary = Dictionary::from_path(dict)?;
    let mut engine = Engine::new(dictionary).with_learner(Box::new(FrequencyLearner::default()));
    if let Some((language, path)) = glossary {
        engine = engine.with_translator(Box::new(Glossary::from_path(language, path)?));
    }
    Ok(engine)
}
