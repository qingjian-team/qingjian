//! 进程级单例：一个 Engine + 一个候选窗口，所有输入会话共用。
//!
//! IMK 的所有回调都在主线程，所以用 `thread_local` 而不是锁；从别的线程访问会拿到 `None`。
//!
//! 配置只有一条通路：[`Host::apply_config`] 把当前 `Config` 推给 Engine 与界面。启动、菜单开关、
//! 设置窗口、手改文件被监视到，全都走它；三个入口都只写 `config.toml`，不各存一套状态。

mod cloud;
mod cloud_test_monitor;
mod config;
mod config_watch;
mod diagnostics;
mod dictionaries;
mod dictionary_info;
mod extra_dictionaries;
mod init;
mod notice;
mod predict_monitor;
mod presenting;
mod session;
mod settings;
mod translation_job;

use std::cell::RefCell;
use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::{NSProcessInfo, NSRect, NSString};
use qingjian_core::{
    Candidate, CandidateKind, Cell, CloudWord, EmojiTable, Engine, FuzzyRules, Language, ModeKeys,
    NoGlossFiller, NoInputLogger, NoPredictor, Prediction, ShuangpinScheme,
};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, InputLog, UsageStats, VocabularyBook};
use qingjian_lm::BigramModel;
use qingjian_platform::{
    AppsConfig, DEFAULT_ENGLISH_CANDIDATES_OFF, DictionariesConfig, KeyCombo, LayoutMode, LogLevel,
    Modifiers, PAGE_KEY_OPTIONS, PreeditMode, ShortcutConfig, ThemeMode,
};
use qingjian_predict::{
    CloudGlossFiller, CloudPredictor, ConnectionTest, PredictConfig, PredictError,
};
use qingjian_translate::{Glossary, LayeredTranslator, LevelTable, PersonalGlossary};

use crate::app::BundleInfo;
use crate::app::{Settings, logging, paths};
use crate::candidates::{CandidateWindow, Frame, Preedit, Row};
use crate::error::HostError;
use crate::menubar::{InputMenu, MenuAction, ModeIndicator};
use crate::preferences::{PreferencesWindow, Setting, SettingValue};

use cloud_test_monitor::CloudTestMonitor;
use config_watch::ConfigWatch;
pub use dictionary_info::DictionaryInfo;
pub use init::init;
use predict_monitor::PredictMonitor;
pub use session::Session;
pub use translation_job::TranslationJob;

pub struct Host {
    /// 输入内核。平台层只能通过它的公开 API 拿候选，不允许碰词库或排序。
    pub engine: Engine,

    /// 候选窗口。
    pub window: CandidateWindow,

    /// 菜单栏的中 / 英状态项。
    pub indicator: ModeIndicator,

    /// 输入法菜单，挂在状态项和系统输入源菜单上。
    pub menu: InputMenu,

    /// 偏好设置窗口。
    pub preferences: PreferencesWindow,

    /// 配置文件的当前值与修改时间。
    pub settings: Settings,

    /// 配置文件监视定时器，激活期间跑。
    pub watch: ConfigWatch,

    /// 上次把学习数据落盘的时间；激活期间的定时器按 [`LEARNING_FLUSH_INTERVAL`] 再刷一次。
    pub last_flush: std::time::Instant,

    /// 当前 Predictor 是按哪份 `[predict]` 建的；配置没变就不重建（重建会起新线程、丢缓存）。
    applied_predict: PredictConfig,

    /// 附加词库是按哪份 `[dictionaries]` 装的；开关变了才重新加载。
    applied_dictionaries: DictionariesConfig,

    /// 偏好设置「词库」页显示的列表，勾选框 / 移除按钮的下标对着它。
    dictionary_list: Vec<DictionaryInfo>,

    /// 当前接在 Engine 上的释义表语言。
    learning_language: Language,

    /// 打进包里的释义表语言，设置窗口按这个顺序列。
    languages: Vec<Language>,

    /// 版本号与构建标识，诊断信息里用。
    version: String,

    /// 见 [`BundleInfo::build`]。
    build: String,

    /// 每页候选数（配置 `[general] page_size`，已夹到 1–9）。
    pub page_size: usize,

    /// 候选窗口第一页末尾留给云端词的格数（配置 `[predict] slots`）。
    pub cloud_slots: usize,

    /// 翻页键对（上一页、下一页）。
    pub page_keys: (char, char),

    /// 配数字键上屏第一 / 第二个译词的修饰键组合（配置 `[shortcut] translation` / `translation_second`）。
    pub translation_keys: (Modifiers, Modifiers),

    /// 配数字键删候选的修饰键（配置 `[shortcut] delete_candidate`）。
    pub delete_keys: Modifiers,

    /// 候选窗口顶行显示的一句临时状态（删了什么词），下一次查询就没了。
    pub status: Option<String>,

    /// 输入日志是否在记（配置 `[general] input_log`），换了才重开文件。
    input_log_enabled: Option<bool>,

    /// 翻译选中文字的快捷键（配置 `[shortcut] translate_selection`）。
    pub translate_keys: KeyCombo,

    /// 进行中的「翻译选中文字」；有它时候选窗口显示的是译文（或「翻译中…」），按键先归它处理。
    pub translation: Option<TranslationJob>,

    /// 正在显示的提示（候选窗口里一行字，几秒后自动收）。
    pub notice: Option<notice::Notice>,

    /// 组句中的拼音显示在行内、候选窗口还是两处。
    pub preedit_mode: PreeditMode,

    /// 英文模式是否给英文候选（配置 `[general] english_candidates`）。
    pub english_candidates: bool,

    /// 按应用的行为（配置 `[apps]`）：哪些应用里英文模式不给候选。
    pub apps: AppsConfig,

    /// 联想结果轮询定时器。
    pub monitor: PredictMonitor,

    /// 进行中的云服务连通性测试（「云服务」页「测试连接」按钮）；没在测为 `None`。
    cloud_test: Option<ConnectionTest>,

    /// 连通性测试的轮询定时器。
    cloud_test_monitor: CloudTestMonitor,

    /// 当前会话的候选、高亮、页码、preedit。
    pub session: Session,

    /// 组句中到达的整句补全，Tab 接受。
    pub sentence: Option<String>,

    /// 最近一次绘制时的光标矩形，联想结果到达后在同一位置重画。
    pub anchor: NSRect,
}

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// 激活期间学习数据最多隔这么久落一次盘。
const LEARNING_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// 输入统计文件名，与学习数据同目录（按天一行，见 `qingjian-learning::UsageStats`）。
const USAGE_FILE: &str = "usage.tsv";

/// 词汇记录文件名，与学习数据同目录（一个译词一行，见 `qingjian-learning::VocabularyBook`）。
const VOCABULARY_FILE: &str = "user-vocab.tsv";

/// 可能打进包里的释义表语言，按这个顺序在设置里列出；文件不存在的不列。
const GLOSSARY_LANGUAGES: [Language; 2] = [Language::English, Language::Japanese];

/// 在单例上执行操作。未初始化、不在主线程、或正处在另一次 `with` 之内（重入）时返回 `None`。
///
/// 重入是真会发生的：闭包里若碰了应用那边的东西（读上下文、取光标位置、插入文字），IMK 会在等应用回话时
/// 跑一轮 run loop，`deactivateServer:` 这类回调就可能在这时进来；这时再 `borrow_mut` 就是 panic 加进程退出。
/// 所以借不到就记一条日志放过这次调用，调用方本来就都按 `None` 处理；根治靠把 IPC 放在借用之外。
pub fn with<R>(f: impl FnOnce(&mut Host) -> R) -> Option<R> {
    HOST.with(|host| match host.try_borrow_mut() {
        Ok(mut guard) => guard.as_mut().map(f),
        Err(_) => {
            tracing::warn!("Host 正在被借用（IMK 回调重入），本次调用跳过");
            None
        }
    })
}
