//! 青简 CLI：Phase 1 的测试工具。
//!
//! 输入拼音，打印候选（词性 + 译文）和各阶段耗时；输入序号上屏并记入用户词频。
//! 不依赖任何平台 API，是 Core 的第一个「壳」。

mod args;
mod cold;
mod display;
mod error;
mod eval;
mod logging;
mod mine;
mod repl;
mod replay;
mod rescoring;
mod tuning;
mod userwords;
mod wordlist;
mod zhiyi;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use qingjian_core::{CodeScheme, EmojiTable, Engine, FuzzyRules, Language, ShuangpinLayout};
use qingjian_dictionary::{
    AuxCodeLookup, AuxCodeTable, CodeTable, Dictionary, GENERAL_DICTIONARY, WordList,
};
use qingjian_learning::FrequencyLearner;
use qingjian_lm::BigramModel;
use qingjian_platform::{Config, Scheme, code_schemes, shuangpin_schemes};
use qingjian_predict::CloudPredictor;
use qingjian_translate::Glossary;

use crate::args::Args;
use crate::error::CliError;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), CliError> {
    dotenvy::dotenv().ok();
    let args = Args::parse();
    let _log_guard = logging::init()?;

    let started = Instant::now();
    let mut engine = build_engine(&args)?;
    tracing::info!(total_ms = started.elapsed().as_millis(), "Engine 就绪");
    engine.set_english_mode(args.english_mode);
    engine.set_chinese_first(args.chinese_first);
    engine.set_show_tones(args.show_tones);
    engine.set_emoji_limit(usize::from(args.emoji_limit));
    tuning::apply(&mut engine, &args.tune)?;
    if let Some(input) = &args.eval_cold {
        cold::run(
            &mut engine,
            input,
            args.cold_output.as_deref().expect("required by clap"),
            args.cold_model.as_deref(),
        )?;
        return Ok(());
    }
    // 查码：只看码表，不查词、不进交互
    if !args.aux_query.is_empty() {
        for word in &args.aux_query {
            let codes: Vec<_> = engine
                .aux_codes()
                .iter()
                .flat_map(|table| table.codes_of(word))
                .collect();
            if codes.is_empty() {
                println!("{word}\t（没有码）");
            } else {
                println!("{word}\t{}", codes.join(" "));
            }
        }
        return Ok(());
    }
    if let Some(path) = &args.mine_log {
        mine::run(&args, path)?;
        return Ok(());
    }
    if args.check_user_words {
        userwords::run(&args, &mut engine)?;
        return Ok(());
    }
    if let Some(path) = &args.import_words {
        wordlist::run(&args, path)?;
        return Ok(());
    }
    if let Some(path) = &args.replay {
        // 日志里形码那些行也要能重放：回放按每条的方案切引擎，所以它自己得留一份码表
        // （`build_engine` 那份的所有权已经交给引擎了）
        let code = args.code.as_deref().map(load_code_scheme).transpose()?;
        let shuangpin = load_shuangpin_scheme(&args)?;
        let mut reranker = args
            .zhiyi
            .as_deref()
            .map(|dir| {
                zhiyi::Reranker::load(
                    dir,
                    args.zhiyi_top,
                    (args.zhiyi_lm_weight, args.zhiyi_rank_weight),
                )
            })
            .transpose()?;
        let report = replay::run(
            &mut engine,
            path,
            args.misses,
            code,
            shuangpin,
            reranker.as_mut(),
        )?;
        print!("{report}");
        if let Some(reranker) = &reranker {
            println!("{}", reranker.summary());
        }
        return Ok(());
    }
    if let Some(model) = &args.eval_generate {
        let report = eval::generate::run(
            &engine,
            &args.eval_text,
            model,
            args.eval_details.as_deref(),
        )?;
        print!("{report}");
        return Ok(());
    }
    if !args.eval_text.is_empty() {
        let mut reranker = args
            .zhiyi
            .as_deref()
            .map(|dir| {
                zhiyi::Reranker::load(
                    dir,
                    args.zhiyi_top,
                    (args.zhiyi_lm_weight, args.zhiyi_rank_weight),
                )
            })
            .transpose()?;
        let report = eval::run(
            &mut engine,
            &args.eval_text,
            args.eval_save.as_deref(),
            args.misses,
            args.eval_details.as_deref(),
            reranker.as_mut(),
        )?;
        print!("{report}");
        if let Some(reranker) = &reranker {
            println!("{}", reranker.summary());
        }
        return Ok(());
    }
    if args.inputs.is_empty() {
        repl::run(&mut engine, args.limit)?;
    } else {
        for input in &args.inputs {
            println!("> {input}");
            if args.typing {
                display::show_typing(&mut engine, input);
            } else {
                display::show(&mut engine, input, args.limit);
            }
        }
    }
    engine.learner_mut().flush();
    Ok(())
}

/// 组装 Engine：这是 Core 之外唯一知道具体 Translator / Learner 类型的地方。
/// `--code` 给的形码：目录按方案读（`scheme.toml` 的规则照用），文件当 86 五笔的码表、规则全关。
fn load_code_scheme(path: &Path) -> Result<CodeScheme, CliError> {
    if path.is_dir() {
        let entry =
            code_schemes::from_dir(path).ok_or_else(|| CliError::CodeScheme(path.to_owned()))?;
        return Ok(entry.load()?);
    }
    Ok(CodeScheme::new("wubi", CodeTable::from_path(path)?))
}

/// `--shuangpin` 给的是目录时按用户双拼方案读；给的是方案写法（`xiaohe`）时为 `None`。
fn load_shuangpin_scheme(args: &Args) -> Result<Option<Arc<ShuangpinLayout>>, CliError> {
    let Some(path) = args
        .shuangpin
        .as_deref()
        .map(Path::new)
        .filter(|path| path.is_dir())
    else {
        return Ok(None);
    };
    let entry = shuangpin_schemes::from_dir(path)
        .ok_or_else(|| CliError::ShuangpinScheme(path.to_owned()))?;
    Ok(Some(entry.layout))
}

fn build_engine(args: &Args) -> Result<Engine, CliError> {
    let language: Language = args
        .language
        .parse()
        .map_err(|_| CliError::Language(args.language.clone()))?;
    if language == Language::Chinese {
        return Err(CliError::Language(args.language.clone()));
    }
    let dict_path = args
        .dict
        .clone()
        .unwrap_or_else(|| args::default_data_file("dict.tsv"));
    let glossary_path = args
        .glossary
        .clone()
        .unwrap_or_else(|| args::default_data_file(&format!("glossary-{}.tsv", language.code())));

    let started = Instant::now();
    let dictionary = Dictionary::from_path(&dict_path)?;
    let dict_load = started.elapsed();
    let started = Instant::now();
    let glossary = Glossary::from_path(language, &glossary_path)?;
    let glossary_load = started.elapsed();
    let english_path = args.english.clone().or_else(|| {
        let path = args::default_data_file("english.tsv");
        path.is_file().then_some(path)
    });
    let started = Instant::now();
    let english = english_path.as_ref().map(WordList::from_path).transpose()?;
    let english_load = started.elapsed();
    let learner = match &args.user_dict {
        Some(path) => FrequencyLearner::from_path(path)?,
        None => FrequencyLearner::default(),
    };
    tracing::info!(
        dict = %dict_path.display(),
        entries = dictionary.len(),
        glossary = %glossary_path.display(),
        glosses = glossary.len(),
        english = english.as_ref().map_or(0, WordList::len),
        learned = learner.len(),
        dict_ms = dict_load.as_millis(),
        glossary_ms = glossary_load.as_millis(),
        english_ms = english_load.as_millis(),
        "加载完成"
    );
    let mut engine = Engine::new(dictionary)
        .with_translator(Box::new(glossary))
        .with_learner(Box::new(learner));
    // 回放按日志逐键重喂：日志里选词之后的退格删的是字母，不是撤回选择
    engine.set_hold_selections(false);
    let general = args
        .general_dict
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("data/generated/dicts/{GENERAL_DICTIONARY}.qj")));
    let mut extras = Vec::new();
    if general.as_os_str() != "none" && general.is_file() {
        let mut dictionary = Dictionary::from_path(&general)?;
        dictionary.set_secondary(true);
        tracing::info!(path = %general.display(), entries = dictionary.len(), "通用词库已加载");
        extras.push(dictionary);
    }
    for path in &args.extra_dict {
        let dictionary = Dictionary::from_path(path)?;
        tracing::info!(path = %path.display(), entries = dictionary.len(), "附加词库已加载");
        extras.push(dictionary);
    }
    if !extras.is_empty() {
        engine.set_extra_dictionaries(extras);
    }
    // 英文候选的中文释义可选
    let zh_glossary = args::default_data_file("glossary-zh.tsv");
    if zh_glossary.is_file() {
        let glossary = Glossary::from_path(Language::Chinese, &zh_glossary)?;
        tracing::info!(glosses = glossary.len(), "英→中释义表已加载");
        engine = engine.with_english_translator(Box::new(glossary));
    }
    if let Some(words) = english {
        engine = engine.with_english(words);
    }
    engine.set_english_in_chinese(!args.no_english_in_chinese);
    // emoji 表随仓库提供（Unicode License）：中文表 + 英文表合成一张，一张都没有就不出 emoji 候选
    let mut emoji: Option<EmojiTable> = None;
    let started = Instant::now();
    for name in ["emoji-zh.tsv", "emoji-en.tsv"] {
        let path = std::path::PathBuf::from("assets/emoji").join(name);
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
        tracing::info!(
            words = table.len(),
            load_ms = started.elapsed().as_millis(),
            "emoji 表已加载"
        );
        engine = engine.with_emoji(table);
    }
    // 语言模型可选：没有就退化成一元词频整句；打包过的 lm.qj 优先
    let packed = std::path::PathBuf::from("data/generated/lm.qj");
    let unigram = std::path::PathBuf::from("data/generated/lm-unigram.tsv");
    let bigram = std::path::PathBuf::from("data/generated/lm-bigram.tsv");
    if packed.is_file() || (unigram.is_file() && bigram.is_file()) {
        let started = Instant::now();
        let model = if packed.is_file() {
            BigramModel::from_path(&packed)?
        } else {
            BigramModel::from_paths(&unigram, &bigram)?
        };
        tracing::info!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            load_ms = started.elapsed().as_millis(),
            "语言模型已加载"
        );
        engine = engine.with_language_model(Box::new(model));
    }
    if let Some(dir) = &args.neural {
        let started = Instant::now();
        let scorer = qingjian_neural::CharScorer::load(dir)?;
        // 与产品端一致：字表里有 <sep> 的是 P2C 模型，按按键打分；没有的是字级模型，按前文打分
        let p2c = scorer.vocab().sep().is_some();
        let scorer: Box<dyn qingjian_core::sentence::SentenceScorer> = if p2c {
            Box::new(qingjian_neural::P2cScorer(scorer))
        } else {
            Box::new(scorer)
        };
        tracing::info!(
            load_ms = started.elapsed().as_millis(),
            weight = args.neural_weight.unwrap_or(qingjian_core::NEURAL_WEIGHT),
            kind = if p2c { "P2C" } else { "字级" },
            "神经重打分已启用"
        );
        engine = if args.neural_async {
            engine.with_async_sentence_scorer(
                scorer,
                args.neural_weight,
                args.neural_margin,
                args.neural_context,
            )
        } else {
            engine.with_sentence_scorer(
                scorer,
                args.neural_weight,
                args.neural_margin,
                args.neural_context,
            )
        };
    }
    if let Some(path) = &args.eval_p2c {
        let scorer = qingjian_neural::CharScorer::load(path)?;
        if scorer.vocab().sep().is_none() {
            return Err(qingjian_neural::NeuralError::Corrupt(
                "Hanzhang Tongbian model requires a <sep> token in its vocabulary",
            )
            .into());
        }
        engine = engine.with_sentence_scorer(
            Box::new(eval::p2c::P2cScorer(scorer)),
            args.neural_weight,
            args.neural_margin,
            None,
        );
    }
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(args::default_config_file);
    let mut config = if args.eval_cold.is_some() {
        let mut isolated = Config::default();
        isolated.predict.enabled = false;
        isolated
    } else {
        Config::load(&config_path)?
    };
    if args.predict {
        config.predict.enabled = true;
    }
    if !args.fuzzy.is_empty() {
        let mut rules = FuzzyRules::default();
        for name in &args.fuzzy {
            if name == "all" {
                rules = FuzzyRules::ALL;
            } else if !rules.enable(name) {
                tracing::warn!(name, "不认识的模糊音规则，忽略");
            }
        }
        config.fuzzy = rules;
    }
    if config.fuzzy.any() {
        tracing::info!(rules = ?config.fuzzy, "模糊音已启用");
    }
    engine.set_traditional(config.general.traditional());
    engine.set_fuzzy(config.fuzzy);
    engine.set_mode_keys(config.shortcut.mode);
    // `--shuangpin` 写的是 [general] scheme，off 就是全拼
    let custom = load_shuangpin_scheme(args)?;
    if let Some(scheme) = args.shuangpin.as_ref().filter(|_| custom.is_none()) {
        config.general.scheme = if scheme == "off" {
            Scheme::Pinyin.key().to_owned()
        } else {
            scheme.clone()
        };
    }
    // 两条轴：拼音侧看 `[general] scheme`，形码侧看 `[general] code`；`--code` 给了码表就算开着形码。
    // 两边都开就是混输，见 docs/user/input/wubi.md
    let scheme = if custom.is_some() {
        Scheme::Custom
    } else {
        config.general.scheme()
    };
    let code = config.general.code().is_some() || args.code.is_some();
    tracing::info!(pinyin = scheme.key(), code, "输入方案已启用");
    match custom {
        Some(layout) => engine.set_shuangpin_layout(Some(layout)),
        None => engine.set_shuangpin(scheme.shuangpin()),
    }
    engine.set_zhuyin_mode(scheme == Scheme::Zhuyin);
    engine.set_nine_key(args.nine_key);
    // 拼音侧关掉且形码开着才是「只用形码」；两边都关着时留拼音兜底
    engine.set_phonetic(scheme.is_on() || !code);
    // 形码的码表由 `--code` 显式给（配置只说「用哪套」，码表文件在哪由壳决定）
    if let Some(path) = &args.code {
        engine.set_code_scheme(Some(load_code_scheme(path)?));
        tracing::info!(table = %path.display(), "形码方案已载入");
    }
    engine.set_aux_code_key(config.general.aux_code_key(), config.general.page_keys());
    engine.set_aux_keep_empty(config.general.aux_code_keep_empty);
    if !args.aux_table.is_empty() {
        let mut tables: Vec<Arc<dyn AuxCodeLookup>> = Vec::new();
        for path in &args.aux_table {
            let table = if path.is_dir() {
                let scheme = qingjian_platform::aux_schemes::from_dir(path)
                    .ok_or_else(|| CliError::AuxScheme(path.to_owned()))?;
                AuxCodeTable::from_path(&scheme.table)?.with_word_rules(scheme.words)
            } else {
                AuxCodeTable::from_path(path)?
            };
            tracing::info!(
                path = %path.display(),
                entries = table.len(),
                words = table.word_count(),
                "辅码码表已加载"
            );
            tables.push(Arc::new(table));
        }
        engine.set_aux_codes(tables);
        // CLI 没有配置开关：给了码表即开辅码（缺省关），replay 统计不哑
        engine.set_aux_enabled(true);
    }
    if config.predict.enabled {
        let predictor = CloudPredictor::new(&config.predict)?;
        engine = engine.with_predictor(Box::new(predictor));
    }
    Ok(engine)
}
