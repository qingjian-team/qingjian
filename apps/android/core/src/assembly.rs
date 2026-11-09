//! 装配 Engine：装的东西与 Linux Server 的 `assembly` 一致；云端由会话另接（`Session::set_cloud`），不带神经重排。
//!
//! `data_dir` 是壳从 APK assets 解出来的随包数据目录（`.qj` 要 mmap，必须是真文件），
//! `user_dir` 是应用私有目录下的学习数据目录。

use std::path::{Path, PathBuf};
use std::time::Instant;

use qingjian_core::{EmojiTable, Engine, Language, Translator};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, PhraseBook, UsageStats, VocabularyBook};
use qingjian_lm::BigramModel;
use qingjian_translate::{Glossary, LayeredTranslator, PersonalGlossary};

use crate::error::AndroidError;

pub fn assemble(data_dir: &Path, user_dir: &Path) -> Result<Engine, AndroidError> {
    let started = Instant::now();
    let dictionary = Dictionary::from_path(packed_or_tsv(data_dir, "dict"))?;
    let learner = load_learner(user_dir);
    tracing::info!(
        entries = dictionary.len(),
        learned = learner.len(),
        dictionary_ms = started.elapsed().as_millis(),
        "词库与学习数据已加载"
    );
    let mut engine = Engine::new(dictionary)
        .with_learner(Box::new(learner))
        .with_usage_meter(Box::new(UsageStats::open(user_dir)))
        .with_vocabulary_tracker(Box::new(VocabularyBook::open(
            user_dir.join("user-vocab.tsv"),
        )));
    // 学习语言的释义表不在这里装：壳接着交设置（`Session::set_options`），按那里的学习语言装
    let english_glossary = data_dir.join("glossary-zh.qj");
    if english_glossary.is_file() {
        match Glossary::from_path(Language::Chinese, &english_glossary) {
            Ok(glossary) => engine = engine.with_english_translator(Box::new(glossary)),
            Err(error) => tracing::warn!(%error, "英→中释义表加载失败"),
        }
    }
    let english = data_dir.join("english.tsv");
    if english.is_file() {
        match WordList::from_path(&english) {
            Ok(words) => engine = engine.with_english(words),
            Err(error) => tracing::warn!(%error, "英文词表加载失败"),
        }
    }
    if let Some(table) = load_emoji(data_dir) {
        engine = engine.with_emoji(table);
    }
    let model = data_dir.join("lm.qj");
    if model.is_file() {
        let started = Instant::now();
        match BigramModel::from_path(&model) {
            Ok(model) => {
                tracing::info!(load_ms = started.elapsed().as_millis(), "语言模型已加载");
                engine = engine.with_language_model(Box::new(model));
            }
            Err(error) => tracing::warn!(%error, "语言模型加载失败，使用词频整句"),
        }
    }
    load_phrases(&mut engine, user_dir);
    Ok(engine)
}

/// 「青简翻译」（系统文字选择菜单）用的轻量 Engine：只为 `request_translation` 定方向（汉字为主译成学习语言、外文译成中文），
/// 所以只挂主词库（mmap，几毫秒）与学习语言的释义表，不装语言模型、英文词表、学习数据。`language` 为 `None`（不显示译文）时译成英语，同 PC。
pub fn assemble_translation(
    data_dir: &Path,
    user_dir: &Path,
    language: Option<Language>,
) -> Result<Engine, AndroidError> {
    let mut engine = Engine::new(Dictionary::from_path(packed_or_tsv(data_dir, "dict"))?);
    if let Some(translator) =
        language.and_then(|language| load_translator(data_dir, user_dir, language))
    {
        engine.set_translator(translator);
    }
    Ok(engine)
}

/// 带输入码的常用语挂成自定义短语（打这串码出这条）；常用语改了之后会话也走这里（[`crate::Session::reload_phrases`]）。
pub(crate) fn load_phrases(engine: &mut Engine, user_dir: &Path) {
    if let Err(error) = engine.set_custom_phrases(PhraseBook::open(user_dir).custom_phrases()) {
        tracing::warn!(%error, "常用语的输入码挂不上");
    }
}

/// 一门学习语言的释义：随包表叠上个人释义表（`user-glossary-<语言>.tsv`）。没有这门语言的表或装不上返回 `None`。
pub(crate) fn load_translator(
    data_dir: &Path,
    user_dir: &Path,
    language: Language,
) -> Option<Box<dyn Translator>> {
    let path = packed_or_tsv(data_dir, &format!("glossary-{}", language.code()));
    if !path.is_file() {
        tracing::warn!(language = language.code(), "没有这门语言的释义表");
        return None;
    }
    match Glossary::from_path(language, &path) {
        Ok(bundled) => {
            let personal = PersonalGlossary::open(
                language,
                user_dir.join(format!("user-glossary-{}.tsv", language.code())),
            );
            Some(Box::new(LayeredTranslator::new(bundled, personal)))
        }
        Err(error) => {
            tracing::warn!(%error, language = language.code(), "释义表加载失败");
            None
        }
    }
}

/// `.qj` 优先，没有就用同名 TSV（开发时随包的是 assets/sample/ 样例）。
fn packed_or_tsv(data_dir: &Path, stem: &str) -> PathBuf {
    let packed = data_dir.join(format!("{stem}.qj"));
    if packed.is_file() {
        packed
    } else {
        data_dir.join(format!("{stem}.tsv"))
    }
}

/// 读不了就退回只在内存里学，不拿空表覆盖用户文件。
fn load_learner(user_dir: &Path) -> FrequencyLearner {
    let path = user_dir.join("user.tsv");
    match FrequencyLearner::from_path(&path) {
        Ok(learner) => learner,
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "学习数据读取失败，本次只在内存里学习");
            FrequencyLearner::default()
        }
    }
}

/// 几张 emoji 表合成一张；坏的跳过。
fn load_emoji(data_dir: &Path) -> Option<EmojiTable> {
    let mut merged: Option<EmojiTable> = None;
    for name in ["emoji-zh.tsv", "emoji-en.tsv"] {
        let path = data_dir.join(name);
        if !path.is_file() {
            continue;
        }
        match EmojiTable::from_path(&path) {
            Ok(table) => match &mut merged {
                Some(all) => all.merge(table),
                None => merged = Some(table),
            },
            Err(error) => tracing::warn!(%error, path = %path.display(), "emoji 表加载失败，跳过"),
        }
    }
    merged
}
