//! Engine 组装：只挂首版 MCP 工具要的件（词库 / 释义表 / 英文词表 / emoji / 学习器），
//! 不挂整句模型与神经重排——`lookup` 的词级候选排序与输入法一致，进程也更轻。
//! 数据文件选择与 `apps/cli` 同一套逻辑（data/generated 优先、仓库随包数据次之、样例兜底），
//! 保证 MCP 查词结果与 CLI 一致。

use std::path::PathBuf;

use qingjian_core::{EmojiTable, Engine, Language};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::FrequencyLearner;
use qingjian_translate::Glossary;

use crate::args::Args;
use crate::error::Error;

/// 按优先级挑一个存在的数据文件：`data/generated/` 里打包好的 `.qj`、那里的 TSV、
/// 仓库自带的产品数据（`assets/lexicon/dict.tsv`、`assets/glossary/glossary-*.tsv`），
/// 最后是 `assets/sample/` 的样例。与 `apps/cli/src/args.rs` 保持同款。
pub fn default_data_file(name: &str) -> PathBuf {
    let generated = PathBuf::from("data/generated").join(name);
    let packed = generated.with_extension("qj");
    let shipped = if name.starts_with("glossary-") {
        PathBuf::from("assets/glossary").join(name)
    } else {
        PathBuf::from("assets/lexicon").join(name)
    };
    for candidate in [packed, generated, shipped] {
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("assets/sample").join(name)
}

/// 组装查词 / 释义要的 Engine。这是 Core 之外唯一知道具体 Translator 类型的地方，
/// 结构与 `apps/cli::main::build_engine` 对齐但去掉了整句 / 神经 / 云端。
pub fn build_engine(args: &Args) -> Result<Engine, Error> {
    let language: Language = args
        .language
        .parse()
        .map_err(|_| Error::Engine(format!("不认识的学习语言 {}", args.language)))?;
    if language == Language::Chinese {
        return Err(Error::Engine("MCP 释义表只面向外语学习，不支持中文".into()));
    }
    let dict_path = args
        .dict
        .clone()
        .unwrap_or_else(|| default_data_file("dict.tsv"));
    let glossary_path = args
        .glossary
        .clone()
        .unwrap_or_else(|| default_data_file(&format!("glossary-{}.tsv", language.code())));

    let dictionary = Dictionary::from_path(&dict_path)?;
    let glossary = Glossary::from_path(language, &glossary_path)?;
    let mut engine = Engine::new(dictionary)
        .with_translator(Box::new(glossary))
        .with_learner(Box::new(FrequencyLearner::default()));

    let english_path = args.english.clone().or_else(|| {
        let path = default_data_file("english.tsv");
        path.is_file().then_some(path)
    });
    if let Some(path) = english_path {
        let words = WordList::from_path(&path)?;
        engine = engine.with_english(words);
    }
    // emoji 表随仓库提供（Unicode License）：中文表 + 英文表合成一张，都没有就不出 emoji 候选
    let mut emoji: Option<EmojiTable> = None;
    for name in ["emoji-zh.tsv", "emoji-en.tsv"] {
        let path = PathBuf::from("assets/emoji").join(name);
        if !path.is_file() {
            continue;
        }
        let table = EmojiTable::from_path(&path)?;
        match &mut emoji {
            Some(all) => all.merge(table),
            None => emoji = Some(table),
        }
    }
    if let Some(table) = emoji {
        engine = engine.with_emoji(table);
    }
    Ok(engine)
}
