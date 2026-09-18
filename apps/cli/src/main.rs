//! 青简 CLI：Phase 1 的测试工具。
//!
//! 输入拼音，打印候选（词性 + 译文）和各阶段耗时；输入序号上屏并记入用户词频。
//! 不依赖任何平台 API，是 Core 的第一个「壳」。

mod args;
mod display;
mod error;
mod eval;
mod logging;
mod repl;
mod replay;
mod rescoring;
mod tuning;

use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use qingjian_core::{EmojiTable, Engine, FuzzyRules, Language};
use qingjian_dictionary::{AuxCodeLookup, AuxCodeTable, CodeTable, Dictionary, WordList};
use qingjian_learning::FrequencyLearner;
use qingjian_lm::BigramModel;
use qingjian_platform::{Config, Scheme};
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
    tuning::apply(&mut engine, &args.tune)?;
    // 查码：只看码表，不查词、不进交互
    if !args.aux_query.is_empty() {
        for word in &args.aux_query {
            let codes: Vec<&str> = engine
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
    if let Some(path) = &args.replay {
        // 日志里形码那些行也要能重放：回放按每条的方案切引擎，所以它自己得留一份码表
        // （`build_engine` 那份的所有权已经交给引擎了）
        let code_table = args.wubi.as_ref().map(CodeTable::from_path).transpose()?;
        let report = replay::run(&mut engine, path, args.misses, code_table)?;
        print!("{report}");
        return Ok(());
    }
    if !args.eval_text.is_empty() {
        let report = eval::run(
            &mut engine,
            &args.eval_text,
            args.eval_save.as_deref(),
            args.misses,
        )?;
        print!("{report}");
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
    if !args.extra_dict.is_empty() {
        let mut extras = Vec::new();
        for path in &args.extra_dict {
            let dictionary = Dictionary::from_path(path)?;
            tracing::info!(path = %path.display(), entries = dictionary.len(), "附加词库已加载");
            extras.push(dictionary);
        }
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
        tracing::info!(
            load_ms = started.elapsed().as_millis(),
            weight = args.neural_weight.unwrap_or(qingjian_core::NEURAL_WEIGHT),
            "神经重打分已启用"
        );
        engine = if args.neural_async {
            engine.with_async_sentence_scorer(
                Box::new(scorer),
                args.neural_weight,
                args.neural_margin,
                args.neural_context,
            )
        } else {
            engine.with_sentence_scorer(
                Box::new(scorer),
                args.neural_weight,
                args.neural_margin,
                args.neural_context,
            )
        };
    }
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(args::default_config_file);
    let mut config = Config::load(&config_path)?;
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
    engine.set_traditional_mode(config.general.traditional);
    engine.set_fuzzy(config.fuzzy);
    engine.set_mode_keys(config.shortcut.mode);
    // `--shuangpin` 现在写的是 [general] scheme（同一个维度的旧键已经并进去），off 就是全拼
    if let Some(scheme) = &args.shuangpin {
        config.general.scheme = if scheme == "off" {
            Scheme::Pinyin.key().to_owned()
        } else {
            scheme.clone()
        };
    }
    // 两条轴：拼音侧看 `[general] scheme`，形码侧看 `[general] wubi`；`--wubi` 给了码表就算开着形码。
    // 两边都开就是混输，见 docs/user/input/fuzzy-and-shuangpin.md
    let scheme = config.general.scheme();
    let wubi = config.general.wubi() || args.wubi.is_some();
    tracing::info!(pinyin = scheme.key(), wubi, "输入方案已启用");
    engine.set_shuangpin(scheme.shuangpin());
    engine.set_zhuyin_mode(scheme == Scheme::Zhuyin);
    // 拼音侧关掉且形码开着才是「只用形码」；两边都关着时留拼音兜底
    engine.set_phonetic(scheme.is_on() || !wubi);
    // 形码的码表由 `--wubi` 显式给（方案本身只说「用哪套」，码表文件在哪由壳决定）
    if let Some(path) = &args.wubi {
        engine.set_code_table(Some(CodeTable::from_path(path)?));
        tracing::info!(table = %path.display(), "形码码表已载入");
    }
    engine.set_aux_code_key(config.general.aux_code_key(), config.general.page_keys());
    engine.set_aux_keep_empty(config.general.aux_code_keep_empty);
    if !args.aux_table.is_empty() {
        let mut tables: Vec<Arc<dyn AuxCodeLookup>> = Vec::new();
        for path in &args.aux_table {
            let table = AuxCodeTable::from_path(path)?;
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
