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
use qingjian_platform::{Config, extra_dictionaries, model_words};
use qingjian_translate::{Glossary, LayeredTranslator, PersonalGlossary};

use crate::error::ServerError;

pub use self::language_model::LanguageModelFiles;
pub use self::spec::AssemblySpec;

/// 知意三元模型在用户目录里的位置（与 `models/zhiyi-onnx` 并列）。
const ZHIYI_NGRAM: &str = "models/zhiyi-ngram/zh.ngram";

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
            .with_usage_meter(Box::new(UsageStats::open(dir)))
            .with_panel_history(Box::new(PanelUsage::open(dir)))
            .with_vocabulary_tracker(Box::new(VocabularyBook::open(dir.join("user-vocab.tsv"))));
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
    let aux_sources = AuxCodeSources {
        bundled_codes: spec.bundled_codes_dir.clone(),
        user_codes: user_codes_dir(spec.user_dir.as_deref()),
        bundled_schemes: spec.bundled_schemes_dir.clone(),
        user_dir: spec.user_dir.clone(),
    };
    engine.set_aux_codes(aux_sources.load(&spec.aux_code));
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
    // 音标表是可选的：随包数据里没有它（比如样例数据）就只是不显示音标，不影响英文候选本身
    if let Some(path) = &spec.english_ipa {
        match std::fs::read_to_string(path) {
            Ok(source) => match WordList::parse_pronunciations(&source) {
                Ok(table) => {
                    tracing::info!(words = table.len(), "英文音标表已加载");
                    engine.set_english_pronunciations(Some(table));
                }
                Err(error) => tracing::warn!(%error, "英文音标表解析失败"),
            },
            Err(error) => tracing::warn!(%error, path = %path.display(), "英文音标表读不了"),
        }
    }
    if let Some(table) = load_emoji(&spec.emoji) {
        tracing::info!(words = table.len(), "emoji 表已加载");
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
            "语言模型已加载"
        );
        bigram = Some(model);
    }
    match (spec.user_dir.as_deref().and_then(load_trigram), bigram) {
        (Some((phrase, lattice)), bigram) => {
            if let Some(dir) = &spec.user_dir {
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

/// 用户放进 `models/zhiyi-ngram/` 的知意三元模型：映射两份，一份给选词重排、一份进整句词图（同一文件，页面共用）。
fn load_trigram(user_dir: &Path) -> Option<(TrigramModel, TrigramModel)> {
    let path = user_dir.join(ZHIYI_NGRAM);
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

/// 用户导入词库目录 `dicts/`，不存在则创建；建不了当没有。
pub fn user_dicts_dir(user_dir: Option<&Path>) -> Option<std::path::PathBuf> {
    let dir = user_dir?.join("dicts");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 用户导入码表目录 `codes/`，不存在则创建；建不了当没有。
pub fn user_codes_dir(user_dir: Option<&Path>) -> Option<std::path::PathBuf> {
    let dir = user_dir?.join("codes");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
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

/// 某语言的释义表：`<root>/data/generated/` 打包过的 `.qj` 优先，否则随 git 的 `assets/glossary/` TSV；都没有为 `None`。
///
/// 启动装配（main）与热加载（swap_translator）都走这里。中文那张是英→中表，缺失本就可选、保持静默；
/// 其他学习语言两张候选都缺时警告一条——只陈述「两张候选都没找到」的查找事实，不断言后果，
/// 因为两种语境后果不同：启动装配时是候选旁不显示译文；热加载时旧译文继续显示（由
/// reload/mod.rs 那条「没有这门语言的释义表，学习语言不变」说明）。
pub fn glossary_file(root: &Path, language: Language) -> Option<PathBuf> {
    let code = language.code();
    let candidates = [
        root.join("data/generated")
            .join(format!("glossary-{code}.qj")),
        root.join("assets/glossary")
            .join(format!("glossary-{code}.tsv")),
    ];
    if let Some(path) = candidates.iter().find(|path| path.is_file()) {
        return Some(path.clone());
    }
    if language != Language::Chinese {
        tracing::warn!(
            language = code,
            qj = %candidates[0].display(),
            tsv = %candidates[1].display(),
            "学习语言的释义表两张候选都没找到"
        );
    }
    None
}

/// 随包释义表叠上个人释义表（`user-glossary-<语言>.tsv`）。启动装配与热加载换语言共用。
pub(crate) fn load_glossary(
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
