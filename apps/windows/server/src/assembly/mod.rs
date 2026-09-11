//! 装配 Engine：Server 进程里唯一知道具体 Translator / Learner 类型的地方，装的东西与 macOS 的 `host::init` 一致。

mod language_model;
mod spec;

use std::path::Path;
use std::time::Instant;

use qingjian_core::{EmojiTable, Engine, Language};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, InputLog, UsageStats, VocabularyBook};
use qingjian_platform::extra_dictionaries;
use qingjian_translate::{Glossary, LayeredTranslator, LevelTable, PersonalGlossary};

use crate::error::ServerError;

pub use self::language_model::LanguageModelFiles;
pub use self::spec::AssemblySpec;

/// 装一个 Engine：词库 + 词频学习 + 释义表（随包 + 个人）+ 统计 / 词汇记录 / 输入日志 + 附加词库 +
/// 英文词表 / 英→中释义 + emoji + 语言模型。
pub fn assemble(spec: &AssemblySpec) -> Result<Engine, ServerError> {
    let started = Instant::now();
    let dictionary = Dictionary::from_path(&spec.dict)?;
    let learner = match &spec.user_dir {
        Some(dir) => load_learner(dir),
        None => FrequencyLearner::default(),
    };
    tracing::info!(
        entries = dictionary.len(),
        learned = learner.len(),
        dictionary_ms = started.elapsed().as_millis(),
        "词库与学习数据已加载"
    );
    let mut engine = Engine::new(dictionary).with_learner(Box::new(learner));
    if let Some((language, path)) = &spec.glossary {
        engine = engine.with_translator(Box::new(load_glossary(
            *language,
            path,
            spec.user_dir.as_deref(),
        )?));
    }
    if let Some(dir) = &spec.user_dir {
        engine = engine
            .with_usage_meter(Box::new(UsageStats::open(dir.join("usage.tsv"))))
            .with_vocabulary_tracker(Box::new(load_vocabulary(dir, spec.levels_dir.as_deref())));
        if spec.input_log {
            let path = dir.join("input-log.jsonl");
            tracing::info!(path = %path.display(), "输入日志开着");
            engine = engine.with_input_logger(Box::new(InputLog::open(path)));
        }
    }
    engine.set_extra_dictionaries(extra_dictionaries::load(
        spec.bundled_dicts_dir.as_deref(),
        user_dicts_dir(spec.user_dir.as_deref()).as_deref(),
        &spec.dictionaries,
    ));
    // 英→中释义表坏了只是英文候选右侧留空，不致命。
    if let Some(path) = &spec.english_glossary {
        match Glossary::from_path(Language::Chinese, path) {
            Ok(glossary) => {
                tracing::info!(glosses = glossary.len(), "英→中释义表已加载");
                engine = engine.with_english_translator(Box::new(glossary));
            }
            Err(error) => tracing::warn!(%error, "英→中释义表加载失败"),
        }
    }
    if let Some(path) = &spec.english {
        let words = WordList::from_path(path)?;
        tracing::info!(words = words.len(), "英文词表已加载");
        engine = engine.with_english(words);
    }
    if let Some(table) = load_emoji(&spec.emoji) {
        tracing::info!(words = table.len(), "emoji 表已加载");
        engine = engine.with_emoji(table);
    }
    if let Some(files) = &spec.language_model {
        let started = Instant::now();
        let model = files.load()?;
        tracing::info!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            load_ms = started.elapsed().as_millis(),
            "语言模型已加载"
        );
        engine = engine.with_language_model(Box::new(model));
    }
    Ok(engine)
}

/// 用户导入的词库目录 `dicts/`，不存在则创建；建不了就当没有。
fn user_dicts_dir(user_dir: Option<&Path>) -> Option<std::path::PathBuf> {
    let dir = user_dir?.join("dicts");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 读用户的学习数据（`user.tsv` 及同目录的用户词 / 选择 / 英文词 / 敲错 / n-gram 表）。
/// 真读不了就退回只在内存里学，不拿空表覆盖用户文件；学习数据出问题不能让 Server 起不来。
fn load_learner(dir: &Path) -> FrequencyLearner {
    let path = dir.join("user.tsv");
    match FrequencyLearner::from_path(&path) {
        Ok(learner) => learner,
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "学习数据读取失败，本次只在内存里学习");
            FrequencyLearner::default()
        }
    }
}

/// 随包释义表叠上用户目录的个人释义表（`user-glossary-<语言>.tsv`，释义兜底写入、可手改）。
fn load_glossary(
    language: Language,
    path: &Path,
    user_dir: Option<&Path>,
) -> Result<LayeredTranslator, ServerError> {
    let bundled = Glossary::from_path(language, path)?;
    let personal = match user_dir {
        Some(dir) => PersonalGlossary::open(
            language,
            dir.join(format!("user-glossary-{}.tsv", language.code())),
        ),
        None => PersonalGlossary::in_memory(language),
    };
    if !personal.is_empty() {
        tracing::info!(
            language = language.code(),
            entries = personal.len(),
            "个人释义表已加载"
        );
    }
    Ok(LayeredTranslator::new(bundled, personal))
}

/// 词汇记录（`user-vocab.tsv`），有等级表就按级统计。
fn load_vocabulary(user_dir: &Path, levels_dir: Option<&Path>) -> VocabularyBook {
    let mut vocabulary = VocabularyBook::open(user_dir.join("user-vocab.tsv"));
    let Some(levels_dir) = levels_dir else {
        return vocabulary;
    };
    for language in [Language::English, Language::Japanese] {
        let path = levels_dir.join(format!("levels-{}.tsv", language.code()));
        if !path.is_file() {
            continue;
        }
        match LevelTable::from_path(&path) {
            Ok(table) => vocabulary = vocabulary.with_levels(language, table),
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "词汇等级表读不了，不分级");
            }
        }
    }
    vocabulary
}

/// 几张 emoji 表合成一张；坏的跳过，一张都没有就不出 emoji 候选。
fn load_emoji(paths: &[std::path::PathBuf]) -> Option<EmojiTable> {
    let mut merged: Option<EmojiTable> = None;
    for path in paths {
        match EmojiTable::from_path(path) {
            Ok(table) => match &mut merged {
                Some(all) => all.merge(table),
                None => merged = Some(table),
            },
            Err(error) => tracing::warn!(%error, path = %path.display(), "emoji 表加载失败，跳过"),
        }
    }
    merged
}
