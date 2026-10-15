//! 装配 Engine：Server 里唯一知道具体 Translator / Learner 类型的地方，装的东西与 macOS 的 `host::init` 一致。

mod language_model;
mod spec;

use std::path::{Path, PathBuf};
use std::time::Instant;

use qingjian_core::{EmojiTable, Engine, Language, VietnameseDictionary};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, InputLog, UsageStats, VocabularyBook};
use qingjian_platform::{Config, code_tables, extra_dictionaries};
use qingjian_translate::{Glossary, LayeredTranslator, LevelTable, PersonalGlossary};

use crate::error::ServerError;

pub use self::language_model::LanguageModelFiles;
pub use self::spec::AssemblySpec;

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
    engine.set_aux_codes(code_tables::load(
        spec.bundled_codes_dir.as_deref(),
        user_codes_dir(spec.user_dir.as_deref()).as_deref(),
        &spec.aux_code,
    ));
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
    if let Some((words, phrases)) = &spec.vietnamese {
        match VietnameseDictionary::from_paths(words, phrases) {
            Ok(dictionary) => engine = engine.with_vietnamese(dictionary),
            Err(error) => tracing::warn!(%error, "越南语词表加载失败"),
        }
    }
    if let Some(glossary) = load_vietnamese_translator(spec.vietnamese_glossary_tsv.as_deref()) {
        engine = engine.with_vietnamese_translator(Box::new(glossary));
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

/// 越南语候选旁显示中文释义：把中→越学习释义 TSV 反向成越→中。
fn load_vietnamese_translator(path: Option<&Path>) -> Option<Glossary> {
    let path = path?;
    let source = std::fs::read_to_string(path).ok()?;
    let reversed = reverse_vietnamese_glossary(&source);
    match Glossary::parse(Language::Chinese, &reversed) {
        Ok(glossary) => {
            tracing::info!(entries = glossary.len(), "越南语→中文释义表已加载");
            Some(glossary)
        }
        Err(error) => {
            tracing::warn!(%error, "越南语→中文释义表加载失败");
            None
        }
    }
}

fn reverse_vietnamese_glossary(source: &str) -> String {
    use std::collections::BTreeMap;

    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in source.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t').map(str::trim);
        let Some(chinese) = fields.next().filter(|word| !word.is_empty()) else {
            continue;
        };
        for sense in fields.filter_map(clean_vietnamese_sense) {
            let words = map.entry(sense).or_default();
            if !words.iter().any(|word| word == chinese) {
                words.push(chinese.to_owned());
            }
        }
    }
    let mut out = String::new();
    out.push_str("# 由 glossary-vi.tsv 反向生成。越南语\t中文\n");
    for (vietnamese, mut chinese_words) in map {
        if !chinese_words.is_empty() {
            chinese_words.sort_by(|a, b| {
                chinese_gloss_priority(a)
                    .cmp(&chinese_gloss_priority(b))
                    .then_with(|| a.chars().count().cmp(&b.chars().count()))
                    .then_with(|| a.cmp(b))
            });
            out.push_str(&vietnamese);
            for word in chinese_words.into_iter().take(2) {
                out.push('\t');
                out.push_str(&word);
            }
            out.push('\n');
        }
    }
    out
}

fn chinese_gloss_priority(chinese: &str) -> u8 {
    if is_modern_everyday_chinese(chinese) {
        return 0;
    }
    if matches!(
        chinese,
        "我" | "你" | "他" | "她" | "它" | "自己" | "想" | "要"
    ) {
        return 5;
    }
    if is_non_modern_chinese_gloss(chinese) {
        return 40;
    }
    if chinese.chars().count() == 1 {
        return 30;
    }
    10
}

fn is_modern_everyday_chinese(chinese: &str) -> bool {
    matches!(
        chinese,
        "我" | "你"
            | "他"
            | "她"
            | "它"
            | "我们"
            | "你们"
            | "他们"
            | "她们"
            | "自己"
            | "老婆"
            | "老公"
            | "朋友"
    )
}

fn is_non_modern_chinese_gloss(chinese: &str) -> bool {
    matches!(
        chinese,
        "吾" | "伲"
            | "俺"
            | "偶"
            | "妇"
            | "妻"
            | "夫"
            | "郎"
            | "妾"
            | "堂客"
            | "婆娘"
            | "娘子"
            | "妻室"
            | "夫君"
            | "夫婿"
            | "相公"
            | "郎君"
            | "鄙人"
            | "小可"
            | "足下"
            | "旁友"
            | "盆友"
            | "阿拉"
    )
}

fn clean_vietnamese_sense(text: &str) -> Option<String> {
    let mut text = text.trim();
    let lower = text.to_ascii_lowercase();
    for prefix in [
        "adj.", "adv.", "art.", "aux.", "clf.", "conj.", "int.", "n.", "num.", "part.", "prep.",
        "pron.", "v.", "phr.",
    ] {
        if lower.starts_with(prefix) {
            text = text[prefix.len()..].trim();
            break;
        }
    }
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
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

/// 词汇记录（`user-vocab.tsv`），有等级表就按级统计。
fn load_vocabulary(user_dir: &Path, levels_dir: Option<&Path>) -> VocabularyBook {
    let mut vocabulary = VocabularyBook::open(user_dir.join("user-vocab.tsv"));
    let Some(levels_dir) = levels_dir else {
        return vocabulary;
    };
    for language in [
        Language::English,
        Language::Japanese,
        Language::Spanish,
        Language::Vietnamese,
        Language::German,
    ] {
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
