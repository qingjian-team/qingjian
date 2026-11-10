//! 装配 Engine：Server 里唯一知道具体 Translator / Learner 类型的地方，装的东西与 macOS 的 `host::init` 一致。

mod language_model;
mod spec;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use qingjian_core::{EmojiTable, Engine, Language};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, InputLog, PanelUsage, UsageStats, VocabularyBook};
use qingjian_lm::{MixedModel, TRIGRAM_WEIGHT, TrigramModel};
use qingjian_platform::code_tables::AuxCodeSources;
use qingjian_platform::pronunciations::load_english_pronunciations;
use qingjian_platform::{Config, UserDirs, extra_dictionaries, model_words};
use qingjian_translate::{Glossary, LayeredTranslator, PersonalGlossary, downloaded_glossary};

use crate::error::ServerError;

pub use self::language_model::LanguageModelFiles;
pub use self::spec::AssemblySpec;

/// 知意三元模型在下载的模型目录里的位置（与 `zhiyi-onnx` 并列）。
const ZHIYI_NGRAM: &str = "zhiyi-ngram/zh.ngram";

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
        engine = engine.with_translator(Box::new(load_glossary(
            *language,
            path,
            spec.user.as_ref().map(UserDirs::learning).as_deref(),
        )?));
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
    let aux_sources = AuxCodeSources {
        bundled_codes: spec.bundled_codes_dir.clone(),
        user_codes: spec.user.as_ref().map(UserDirs::code_tables),
        bundled_schemes: spec.bundled_schemes_dir.clone(),
        user_dir: spec.user.as_ref().map(UserDirs::custom),
    };
    engine.set_aux_codes(aux_sources.load(&spec.aux_code));
    if let Some(glossary) = spec
        .english_glossary
        .as_deref()
        .and_then(load_english_glossary)
    {
        engine = engine.with_english_translator(Box::new(glossary));
    }
    if let Some(path) = &spec.english {
        let words = WordList::from_path(path)?;
        tracing::info!(words = words.len(), "english word list loaded");
        engine = engine.with_english(words);
    }
    if let Some(path) = &spec.english_ipa {
        engine.set_english_pronunciations(load_english_pronunciations(path));
    }
    if let Some(table) = load_emoji(&spec.emoji) {
        tracing::info!(words = table.len(), "emoji table loaded");
        engine = engine.with_emoji(table);
    }
    let mut bigram = None;
    if let Some(files) = &spec.language_model {
        let started = Instant::now();
        let model = files.load()?;
        tracing::info!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            load_ms = started.elapsed().as_millis(),
            "language model loaded"
        );
        bigram = Some(model);
    }
    let models = spec.user.as_ref().map(UserDirs::models);
    match (models.as_deref().and_then(load_trigram), bigram) {
        (Some((phrase, lattice)), bigram) => {
            if let Some(dir) = &models {
                let words = load_model_words(&engine, &phrase, &dir.join(ZHIYI_NGRAM), &spec.dict);
                engine.set_model_words(words);
            }
            engine.set_phrase_model(Some(Arc::new(phrase)));
            engine = engine.with_language_model(match bigram {
                Some(bigram) => Box::new(MixedModel::new(
                    Box::new(lattice),
                    Box::new(bigram),
                    TRIGRAM_WEIGHT,
                )),
                None => Box::new(lattice),
            });
        }
        (None, Some(bigram)) => engine = engine.with_language_model(Box::new(bigram)),
        (None, None) => {}
    }
    Ok(engine)
}

/// 用户放进 `downloads\models\zhiyi-ngram\` 的知意三元模型：映射两份，一份给选词重排、一份进整句词图（同一文件，页面共用）。
fn load_trigram(models: &Path) -> Option<(TrigramModel, TrigramModel)> {
    let path = models.join(ZHIYI_NGRAM);
    if !path.is_file() {
        return None;
    }
    let started = Instant::now();
    match TrigramModel::open(&path).and_then(|a| Ok((a, TrigramModel::open(&path)?))) {
        Ok((phrase, lattice)) => {
            tracing::info!(
                unigrams = phrase.unigram_count(),
                trigrams = phrase.trigram_count(),
                load_ms = started.elapsed().as_millis(),
                "三元模型已加载"
            );
            Some((phrase, lattice))
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "三元模型加载失败，整句只用二元模型");
            None
        }
    }
}

/// 三元模型词表里词库没有的词（摸鱼、老梗），作补充词库：缓存在模型旁边，模型或主词库换了才重建（第一次约一秒）。
/// 判断哪些词已有时参照主词库与已装的附加词库；戳只看模型与主词库：导入一本词库不该让缓存作废。
fn load_model_words(
    engine: &Engine,
    model: &TrigramModel,
    model_path: &Path,
    dict: &Path,
) -> Option<Dictionary> {
    let stamp = model_words::stamp(&[model_path, dict]);
    model_words::load(&model_words::cache_path(model_path), &stamp, || {
        let mut references = vec![engine.dictionary()];
        references.extend(engine.extra_dictionaries());
        qingjian_dictionary::supplement(model.vocabulary(), &references)
            .inspect_err(|error| tracing::warn!(%error, "三元模型词表补词失败"))
            .ok()
    })
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

/// 配置里的学习语言；`off` 为 `None`（不显示译文），写得不认识按英文。
pub fn learning_language(config: &Config) -> Option<Language> {
    if config.general.learning_language_off() {
        return None;
    }
    let code = &config.general.learning_language;
    Some(code.parse().unwrap_or_else(|_| {
        tracing::warn!(code, "不认识的学习语言，按英文");
        Language::English
    }))
}

/// 某语言的释义表：设置页下载的（`downloads\glossaries\`，`downloads` 是下载目录）优先，其次 `<root>/data/generated/` 打包过的 `.qj`，
/// 再次同一目录里开发时导出的 TSV；都没有为 `None`。
///
/// 启动装配（main）与热加载（swap_translator）都走这里。中文那张是英→中表，缺失本就可选、保持静默；
/// 其他学习语言几处都缺时警告一条——只陈述查找事实，不断言后果，
/// 因为两种语境后果不同：启动装配时是候选旁不显示译文；热加载时旧译文继续显示（由
/// reload/mod.rs 那条「没有这门语言的释义表，学习语言不变」说明）。
pub fn glossary_file(root: &Path, downloads: Option<&Path>, language: Language) -> Option<PathBuf> {
    let code = language.code();
    let candidates = [
        downloads.map(|dir| downloaded_glossary(dir, language)),
        Some(
            root.join("data/generated")
                .join(format!("glossary-{code}.qj")),
        ),
        Some(
            root.join("data/generated")
                .join(format!("glossary-{code}.tsv")),
        ),
    ];
    if let Some(path) = candidates.iter().flatten().find(|path| path.is_file()) {
        return Some(path.clone());
    }
    if language != Language::Chinese {
        tracing::warn!(
            language = code,
            bundled = %root.join("data/generated").display(),
            "no glossary found for the learning language"
        );
    }
    None
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

/// 随包释义表叠上学习数据目录里的个人释义表（`user-glossary-<语言>.tsv`）。启动装配与热加载换语言共用。
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
