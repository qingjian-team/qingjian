//! 装配 Engine：Server 里唯一知道具体 Translator / Learner 类型的地方，装的东西与 macOS 的 `host::init` 一致。

mod cloud;
mod language_model;
mod spec;

use std::path::{Path, PathBuf};
use std::time::Instant;

use qingjian_core::{EmojiTable, Engine, Language};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, InputLog, PanelUsage, UsageStats, VocabularyBook};
use qingjian_platform::pronunciations::load_english_pronunciations;
use qingjian_platform::{Config, UserDirs, extra_dictionaries};
use qingjian_translate::{Glossary, LayeredTranslator, PersonalGlossary, downloaded_glossary};

use crate::error::ServerError;

pub use self::cloud::attach_cloud;
pub use self::language_model::LanguageModelFiles;
pub use self::spec::AssemblySpec;

pub fn assemble(spec: &AssemblySpec) -> Result<Engine, ServerError> {
    let started = Instant::now();
    let dictionary = Dictionary::from_path(&spec.dict)?;
    let learner = match &spec.user {
        Some(user) => load_learner(&user.learning()),
        None => FrequencyLearner::default(),
    };
    tracing::info!(
        entries = dictionary.len(),
        learned = learner.len(),
        dictionary_ms = started.elapsed().as_millis(),
        "dictionary and learning data loaded"
    );
    let mut engine = Engine::new(dictionary).with_learner(Box::new(learner));
    if let Some((language, path)) = &spec.glossary {
        match load_glossary(
            *language,
            path,
            spec.user.as_ref().map(UserDirs::learning).as_deref(),
        ) {
            Ok(glossary) => engine = engine.with_translator(Box::new(glossary)),
            Err(error) => tracing::warn!(%error, "glossary failed to load, continuing without it"),
        }
    }
    if let Some(user) = &spec.user {
        let records = user.records();
        engine = engine
            .with_usage_meter(Box::new(UsageStats::open(&records)))
            .with_panel_history(Box::new(PanelUsage::open(&records)))
            .with_vocabulary_tracker(Box::new(VocabularyBook::open(
                records.join("user-vocab.tsv"),
            )));
        if spec.input_log {
            let path = user.input_log();
            tracing::info!(path = %path.display(), "input log enabled");
            engine = engine.with_input_logger(Box::new(InputLog::open(path)));
        }
    }
    let user_dicts = spec.user.as_ref().map(UserDirs::extra_dict_dirs);
    let user_dicts: Vec<&Path> = user_dicts.iter().flatten().map(PathBuf::as_path).collect();
    engine.set_extra_dictionaries(extra_dictionaries::load(
        spec.bundled_dicts_dir.as_deref(),
        &user_dicts,
        &spec.dictionaries,
    ));
    engine.set_blocked_words(spec.dictionaries.blocked.clone());
    engine.set_base_words(spec.dictionaries.base);
    if let Some(glossary) = spec
        .english_glossary
        .as_deref()
        .and_then(load_english_glossary)
    {
        engine = engine.with_english_translator(Box::new(glossary));
    }
    if let Some(path) = &spec.english {
        match WordList::from_path(path) {
            Ok(words) => engine = engine.with_english(words),
            Err(error) => tracing::warn!(%error, "english word list failed to load"),
        }
    }
    if let Some(path) = &spec.english_ipa {
        engine.set_english_pronunciations(load_english_pronunciations(path));
    }
    if let Some(table) = load_emoji(&spec.emoji) {
        tracing::info!(words = table.len(), "emoji table loaded");
        engine = engine.with_emoji(table);
    }
    if let Some(files) = &spec.language_model {
        let started = Instant::now();
        match files.load() {
            Ok(model) => {
                tracing::info!(
                    load_ms = started.elapsed().as_millis(),
                    "language model loaded"
                );
                engine = engine.with_language_model(Box::new(model));
            }
            Err(error) => {
                tracing::warn!(%error, "language model failed to load, using word frequencies for sentences")
            }
        }
    }
    Ok(engine)
}

/// 读不了就退回只在内存里学，不拿空表覆盖用户文件。
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

/// `[general] learning_language`，关着为 `None`；不认识的代码按英文。启动装配与热加载共用。
pub fn learning_language(config: &Config) -> Option<Language> {
    if config.general.learning_language_off() {
        return None;
    }
    Some(
        config
            .general
            .learning_language
            .parse()
            .unwrap_or(Language::English),
    )
}

/// 某语言的释义表：设置页下载的（`<用户目录>/glossaries/`）优先，其次 `<root>/data/generated/` 打包过的 `.qj`，
/// 再次同一目录里开发时导出的 TSV；都没有为 `None`。
pub fn glossary_file(root: &Path, downloads: Option<&Path>, language: Language) -> Option<PathBuf> {
    let code = language.code();
    [
        downloads.map(|dir| downloaded_glossary(dir, language)),
        Some(
            root.join("data/generated")
                .join(format!("glossary-{code}.qj")),
        ),
        Some(
            root.join("data/generated")
                .join(format!("glossary-{code}.tsv")),
        ),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.is_file())
}

/// 英→中释义表（英文候选右侧的中文）：装不上记日志返回 `None`。启动装配与下载后重装共用。
pub fn load_english_glossary(path: &Path) -> Option<Glossary> {
    match Glossary::from_path(Language::Chinese, path) {
        Ok(glossary) => {
            tracing::info!(glosses = glossary.len(), "英→中释义表已加载");
            Some(glossary)
        }
        Err(error) => {
            tracing::warn!(%error, "英→中释义表加载失败");
            None
        }
    }
}

/// 随包释义表叠上个人释义表（`user-glossary-<语言>.tsv`）。启动装配与热加载换语言共用。
pub(crate) fn load_glossary(
    language: Language,
    path: &Path,
    learning: Option<&Path>,
) -> Result<LayeredTranslator, ServerError> {
    let bundled = Glossary::from_path(language, path)?;
    let personal = match learning {
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

/// 几张 emoji 表合成一张；坏的跳过。
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
