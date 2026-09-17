//! Linux 壳的业务侧：引擎装配 + 输入会话状态。照搬 macOS host 的结构
//! （init.rs 的装配、session.rs 的分页会话），砍掉首版不做的部分（云/模型/统计）。

mod config_watch;
mod keys;
mod model;
mod paths;
mod session;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use qingjian_core::{CandidateLayout, EmojiTable, Engine, Language};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, UsageStats, VocabularyBook};
use qingjian_lm::BigramModel;
use qingjian_platform::{Config, Modifiers};
use qingjian_translate::{Glossary, LevelTable};

pub use paths::{config_path, data_dir};
use paths::{find_data, find_file, load_glossary};

/// 云端词占位数：Linux 首版无云联想，不留位。
const CLOUD_SLOTS: usize = 0;
/// 学习数据落盘的最小间隔（上屏路径上顺带检查，焦点切换仍即时落）。
const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);
/// 配置探测间隔：按键路径上顺带查 mtime，改完配置敲下一个键就生效。
const CONFIG_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

pub struct Host {
    pub engine: Engine,
    /// 当前查询的候选排布与 UI 状态。
    pub layout: CandidateLayout,
    pub highlighted: usize,
    pub page: usize,
    /// 候选窗顶部的拼音行（带音节分隔）与光标（字节偏移）。
    pub preedit: String,
    pub preedit_cursor: usize,
    /// 待上屏文本：按键处理里攒，shim 每个事件后取走。
    pub pending_commit: Option<String>,
    /// Shift 轻点检测：按下 Shift 后没夹别的键，松开才算「轻点」，切中英。
    shift_armed: bool,

    /// 按下被吞掉、还没等到松键的 keysym：松键按它对称吞。
    /// 判据不能用「当前是否组句」——上屏类的键按下就结束了组句，那样判会漏无头 keyup 给应用。
    swallowed_presses: Vec<u32>,
    /// 本轮查询里是否用方向键/翻页动过高亮。英文模式的空格只在动过之后才选高亮词，
    /// 没动过就原样上屏（打词表里没有的英文词不被补全替换）——照 macOS 语义。
    navigated: bool,
    /// 每页候选数（配置 1–9）。
    page_size: usize,
    last_flush: std::time::Instant,
    /// 配置热加载：watch 的文件、上次见到的修改时间、上次探测时刻。
    config_file: Option<PathBuf>,
    config_mtime: Option<std::time::SystemTime>,
    last_config_check: std::time::Instant,
    /// 当前生效的学习语言（换语言要重载释义表，记着才能比对）。
    learning_language: Option<Language>,
    /// 译词快捷键的两组修饰键（数字键配它：上屏第一/第二个译词）。
    translation_mods: (Modifiers, Modifiers),
    /// 删候选快捷键的修饰键组合（数字键配它，缺省 Shift）。
    delete_mods: Modifiers,
    /// 翻页键对（配置 `[general] page_keys`，缺省 `[` `]`）。
    page_keys: (char, char),
    /// 英文模式给不给候选（配置 `[general] english_candidates`）；关掉就是纯直通。
    english_candidates: bool,
    /// 按应用关英文候选的名单（配置 `[apps] english_candidates_off`）。
    apps: qingjian_platform::AppsConfig,
    /// 当前应用（fcitx5 的 program 名，shim 在变化时告知）。
    program: String,
    /// 拼音行显示位置（配置 `[general] preedit`），shim 按它画。
    preedit_mode: qingjian_platform::PreeditMode,
    /// 输入日志当前开关（热加载时变了才换 logger，与 macOS 同款判等）。
    input_log_enabled: Option<bool>,
    /// 本地整句模型的后台加载回执；None = 没在加载。
    model_loader: Option<model::ModelLoader>,
    /// 配置 `[model] enabled` 当前生效值（变了才装/卸）。
    model_enabled: bool,
    /// 重排防抖截止：到点把攒着的整句路径送后台打分。
    rescore_deadline: Option<std::time::Instant>,
    /// 本轮开始等重排结果的时间（超时兜底）。
    rescore_since: Option<std::time::Instant>,
    data_dir: PathBuf,
}

impl Host {
    /// `config` 传 None = 从标准路径读（测试传 Some 以隔离环境）。
    pub fn init(dir: PathBuf, config: Option<Config>) -> Result<Self, String> {
        let config = config.unwrap_or_else(|| match config_path() {
            Some(path) => Config::load(&path).unwrap_or_else(|error| {
                // 配置笔误不能让输入法起不来：报日志、按默认跑，用户修好重启即生效。
                tracing::error!(%error, "配置解析失败,本次按默认配置");
                Config::default()
            }),
            None => Config::default(),
        });
        let dict_path = find_data(&dir, "dict")
            .ok_or_else(|| format!("{} 下没有 dict.qj/dict.tsv", dir.display()))?;
        let dictionary =
            Dictionary::from_path(&dict_path).map_err(|e| format!("词库加载失败:{e}"))?;
        let mut engine = Engine::new(dictionary);
        let learning_language = match load_glossary(&dir, &config.general.learning_language) {
            Some((language, glossary)) => {
                engine = engine.with_translator(Box::new(glossary));
                Some(language)
            }
            None => {
                tracing::info!("无释义表,候选无译文");
                None
            }
        };
        let learner = FrequencyLearner::from_path(dir.join("user.tsv")).unwrap_or_default();
        engine = engine.with_learner(Box::new(learner));
        // 英文模式的词表与英→中释义，都可选缺：缺了英文模式只是没候选。
        if let Some(path) = find_data(&dir, "english") {
            match WordList::from_path(&path) {
                Ok(words) => engine = engine.with_english(words),
                Err(error) => tracing::warn!(%error, "英文词表加载失败"),
            }
        }
        if let Some(path) = find_data(&dir, "glossary-zh") {
            match Glossary::from_path(Language::Chinese, &path) {
                Ok(glossary) => engine = engine.with_english_translator(Box::new(glossary)),
                Err(error) => tracing::warn!(%error, "英→中释义表加载失败"),
            }
        }
        // 语言模型可选：没有就退化成一元词频整句（打包数据带 lm.qj）。
        if let Some(path) = find_data(&dir, "lm") {
            match BigramModel::from_path(&path) {
                Ok(model) => {
                    tracing::info!(
                        words = model.word_count(),
                        bigrams = model.bigram_count(),
                        "语言模型已加载"
                    );
                    engine = engine.with_language_model(Box::new(model));
                }
                Err(error) => tracing::warn!(%error, "语言模型加载失败,按一元词频整句"),
            }
        }
        // emoji 表（中文、英文）合成一张；一张都没有就不出 emoji 候选。
        let mut emoji: Option<EmojiTable> = None;
        for name in ["emoji-zh.tsv", "emoji-en.tsv"] {
            let Some(path) = find_file(&dir, name) else {
                continue;
            };
            match EmojiTable::from_path(&path) {
                Ok(table) => match &mut emoji {
                    Some(all) => all.merge(table),
                    None => emoji = Some(table),
                },
                Err(error) => tracing::warn!(%error, name, "emoji 表加载失败,跳过"),
            }
        }
        if let Some(table) = emoji {
            tracing::info!(words = table.len(), "emoji 表已加载");
            engine = engine.with_emoji(table);
        }
        // 输入统计与词汇记录（统计页的数据源）；词汇等级表可选，有就按级统计。
        let mut vocabulary = VocabularyBook::open(dir.join("user-vocab.tsv"));
        for (language, file) in [
            (Language::English, "levels-en.tsv"),
            (Language::Japanese, "levels-ja.tsv"),
        ] {
            let Some(path) = find_file(&dir, file) else {
                continue;
            };
            match LevelTable::from_path(&path) {
                Ok(table) => vocabulary = vocabulary.with_levels(language, table),
                Err(error) => tracing::warn!(%error, file, "词汇等级表读不了,不分级"),
            }
        }
        engine = engine
            .with_usage_meter(Box::new(UsageStats::open(dir.join("usage.tsv"))))
            .with_vocabulary_tracker(Box::new(vocabulary));
        let page_size = config.general.page_size();
        let mut host = Host {
            engine,
            layout: CandidateLayout::new(Vec::new(), page_size, CLOUD_SLOTS),
            highlighted: 0,
            page: 0,
            preedit: String::new(),
            preedit_cursor: 0,
            pending_commit: None,
            shift_armed: false,
            swallowed_presses: Vec::new(),
            navigated: false,
            page_size,
            last_flush: std::time::Instant::now(),
            config_file: None,
            config_mtime: None,
            last_config_check: std::time::Instant::now(),
            learning_language,
            translation_mods: config.shortcut.translation_keys(),
            delete_mods: config.shortcut.delete_keys(),
            page_keys: config.general.page_keys(),
            english_candidates: config.general.english_candidates,
            apps: config.apps.clone(),
            program: String::new(),
            preedit_mode: config.general.preedit,
            input_log_enabled: None,
            model_loader: None,
            model_enabled: false,
            rescore_deadline: None,
            rescore_since: None,
            data_dir: dir,
        };
        host.apply_config(&config);
        Ok(host)
    }
}
