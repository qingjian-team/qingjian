//! 进程级单例：一个 Engine + 一个候选窗口，所有输入会话共用。
//!
//! IMK 的所有回调都在主线程，所以用 `thread_local` 而不是锁；从别的线程访问会拿到 `None`。
//!
//! 配置只有一条通路：[`Host::apply_config`] 把当前 `Config` 推给 Engine 与界面。启动、菜单开关、
//! 设置窗口、手改文件被监视到，全都走它；三个入口都只写 `config.toml`，不各存一套状态。

mod aux_code;
mod cloud;
mod config;
mod diagnostics;
mod dictionaries;
mod init;
mod mode;
mod model;
mod presenting;
mod session;
mod settings;

use std::cell::RefCell;
use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::{NSProcessInfo, NSRect, NSString};
use qingjian_core::{
    Candidate, CandidateKind, Cell, CloudWord, EmojiTable, Engine, FuzzyRules, Language, ModeKeys,
    NoGlossFiller, NoInputLogger, NoPredictor, NoTranslator, Prediction, Sense, TelexComposer,
    Translation, Translator,
};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, InputLog, PanelUsage, UsageStats, VocabularyBook};
use qingjian_lm::BigramModel;
use qingjian_platform::extra_dictionaries;
use qingjian_platform::logs::CompositionTrace;
use qingjian_platform::{
    Appearance, AppsConfig, AuxCodeConfig, CandidateRenderer, DEFAULT_ENGLISH_CANDIDATES_ON,
    DictionariesConfig, GeneralConfig, HighlightKeys, KeyCombo, KeyHintMode, LEARNING_LANGUAGE_OFF,
    LayoutMode, LocalModelConfig, LogLevel, Modifiers, PAGE_KEY_OPTIONS, PageKeys, PreeditMode,
    Scheme, ShortcutConfig, UpdateChannel,
};
use qingjian_predict::{
    API_USAGE_FILE, CloudGlossFiller, CloudPredictor, ConnectionTest, PredictConfig, PredictError,
};
use qingjian_translate::{Glossary, LayeredTranslator, LevelTable, PersonalGlossary};

use crate::app::BundleInfo;
use crate::app::{Settings, logging, paths};
use crate::candidates::{CandidateWindow, Frame, Preedit, Row};
use crate::error::HostError;
use crate::menubar::{InputMenu, MenuAction, ModeIndicator};
use crate::preferences::{PreferencesWindow, Setting, SettingValue, UpdateStatus};

use cloud::{CloudTestMonitor, PredictMonitor};
use config::{ConfigWatch, TextReplacement};
pub use dictionaries::DictionaryInfo;
pub use init::init;
use mode::{ModeOsd, ModeSwitch};
use model::RescoreMonitor;
pub use presenting::TranslationJob;
use presenting::{CandidateTranslation, KeyHintState, Notice};
pub use session::Session;

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

    /// 单击 Shift 切中英（勾了 `[shortcut] switch_mode` 的 shift 时）；定时器只在激活期间跑。
    pub mode_switch: ModeSwitch,

    /// 越南语 Simple Telex 的当前词；只在双击 Shift 切出的第三输入法中使用。
    pub vietnamese: TelexComposer,

    /// 由随包越南语释义表派生的输入候选词表。
    pub vietnamese_words: Option<WordList>,

    /// 越南语候选的中文释义：个人模型译词优先，随包反向词库兜底。
    pub vietnamese_glossary: Option<LayeredTranslator>,

    /// 中 / 英与大小写切换时右上角一闪而过的提示。
    pub osd: ModeOsd,

    /// 上一次已经提示过的中英模式；`None` 表示还没提示过（启动后第一次只记不弹）。
    last_osd_english: Option<bool>,

    /// 上一次已经提示过的越南语模式；用于只在进入 / 退出时提示一次。
    last_osd_vietnamese: Option<bool>,

    /// 上一次已经提示过的 Caps Lock 状态，同上。
    last_osd_caps: Option<bool>,

    /// 内置主题与用户主题目录里的主题；激活期间随配置定时器一起看目录有没有变。
    pub themes: qingjian_render::ThemeLibrary,

    /// 上次把学习数据落盘的时间；激活期间的定时器按 [`LEARNING_FLUSH_INTERVAL`] 再刷一次。
    pub last_flush: std::time::Instant,

    /// 当前 Predictor 是按哪份 `[predict]` 建的；配置没变就不重建（重建会起新线程、丢缓存）。
    applied_predict: PredictConfig,

    /// 附加词库是按哪份 `[dictionaries]` 装的；开关变了才重新加载。
    applied_dictionaries: DictionariesConfig,

    /// 辅码码表是按哪份 `[aux_code]` 装的；还没装过为 `None`。
    applied_aux_code: Option<AuxCodeConfig>,

    /// 上次装码表时用户 `codes/` 与 `schemes/` 的文件快照：设置程序导入或改了方案，下一次看配置时就重装。
    aux_files: Vec<(PathBuf, Option<std::time::SystemTime>, u64)>,

    /// 上次装形码方案时用户 `code_dicts/` 的文件快照：设置程序导入或移除了五笔词库，下一次看配置时就重装方案。
    code_word_files: Vec<(PathBuf, Option<std::time::SystemTime>, u64)>,

    /// 上次装附加词库时用户 `dicts/` 的文件快照：设置程序导入的词库不一定改配置（本来就没关着），靠它发现新文件。
    dictionary_files: Vec<(PathBuf, Option<std::time::SystemTime>, u64)>,

    /// 候选词后面带上辅码（配置 `[general] aux_code_show`）。
    pub aux_code_show: bool,

    /// 偏好设置「词库」页显示的列表，勾选框 / 移除按钮的下标对着它。
    dictionary_list: Vec<DictionaryInfo>,

    /// 当前学习语言；`None` 为关（不显示译文）。
    learning_language: Option<Language>,

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

    /// 和主翻页键一起用的额外翻页键（配置 `[shortcut] extra_page_keys`）。
    pub extra_page_keys: PageKeys,

    /// 组句中敲会转全角的标点先把高亮候选上屏、再补该标点（配置 `[general] punct_commits`，缺省关）。
    pub punct_commits: bool,

    /// 装着的形码方案（名字给主题显示），配置没动它时不重读码表。
    pub scheme_slot: qingjian_platform::SchemeSlot,

    /// 配数字键上屏第一 / 第二个译词的修饰键组合（配置 `[shortcut] translation` / `translation_second`）。
    pub translation_keys: (Modifiers, Modifiers),

    /// 候选窗口顶行显示的一句临时状态（删了什么词），下一次查询就没了。
    pub status: Option<String>,

    /// 输入日志是否在记（配置 `[general] input_log`），换了才重开文件。
    input_log_enabled: Option<bool>,

    /// 翻译选中文字的快捷键（配置 `[shortcut] translate_selection`）。
    pub translate_keys: KeyCombo,

    /// 切换全半角标点的组合键（配置 `[shortcut] toggle_punctuation`，缺省没有）。
    pub punctuation_keys: Option<KeyCombo>,

    /// 组句中按需联想的组合键（配置 `[shortcut] predict`）。
    pub predict_keys: KeyCombo,

    /// 在飞的按需联想请求序号：结果到了排在第一页最前面，自动联想的照旧补在末尾。
    manual_prediction: Option<u64>,

    /// 用户从候选菜单发起、正在等待的模型译文生成。
    candidate_translation: Option<CandidateTranslation>,

    /// 进行中的「翻译选中文字」；有它时候选窗口显示的是译文（或「翻译中…」），按键先归它处理。
    pub translation: Option<TranslationJob>,

    /// 正在显示的提示（候选窗口里一行字，几秒后自动收）。
    pub notice: Option<Notice>,

    /// 双拼键位提示什么时候弹出（配置 `[general] key_hint`）。
    pub key_hint_mode: KeyHintMode,

    /// 「卡住时显示」的停顿计时与这一段组句里弹没弹过。
    key_hint: KeyHintState,

    /// 五笔字根图换成的键盘，第一次画时建好（字形解析一次）。
    root_keyboard: Option<qingjian_render::Keyboard>,

    /// 组句中的拼音显示在行内、候选窗口还是两处。
    pub preedit_mode: PreeditMode,

    /// 译词旁是否画词性（配置 `[general] translation_pos`）。
    pub translation_pos: bool,

    /// 候选窗口竖排 / 横排（配置 `[general] layout`）。
    pub layout: LayoutMode,

    /// 横排时上下键展开成矩阵的开关（配置 `[general] horizontal_grid`，缺省关）。
    pub horizontal_grid: bool,

    /// 横排时翻页键也把单行展开成矩阵（配置 `[general] grid_page_keys`，缺省关）。
    pub grid_page_keys: bool,

    /// 候选高亮由哪对方向键移动（配置 `[shortcut] highlight_keys`）。
    pub highlight_keys: HighlightKeys,

    /// 组句中额外使用 Ctrl + F / B / N / P / A / E 导航候选。
    pub extra_candidate_navigation: bool,

    /// 英文模式是否给英文候选（配置 `[general] english_candidates`）。
    pub english_candidates: bool,

    /// 上次从系统读到的文本替换（激活输入法时重读），`[general] system_text_replacements` 开着时并进自定义短语。
    text_replacements: Vec<TextReplacement>,

    /// 按应用的行为（配置 `[apps]`）：哪些应用里英文模式不给候选。
    pub apps: AppsConfig,

    /// 联想结果轮询定时器。
    pub monitor: PredictMonitor,

    /// 进行中的云服务连通性测试（「云服务」页「测试连接」按钮）；没在测为 `None`。
    cloud_test: Option<ConnectionTest>,

    /// 连通性测试的轮询定时器。
    cloud_test_monitor: CloudTestMonitor,

    /// 本地整句模型的防抖与轮询定时器。
    rescore: RescoreMonitor,

    /// 正在后台加载的模型；加载完接到 Engine 上就清掉。
    model_loader:
        Option<std::sync::mpsc::Receiver<Result<model::LoadedModel, qingjian_neural::NeuralError>>>,

    /// 上次套用的 `[model]`，变了才重载 / 卸载。
    applied_model: Option<LocalModelConfig>,

    /// 检查更新；拿不到数据目录时没有。
    updates: Option<qingjian_update::Checker>,

    /// 菜单与「关于」页上正显示的更新状态，变了才刷界面。
    update_status: UpdateStatus,

    /// 当前会话的候选、高亮、页码、preedit。
    pub session: Session,

    /// 正在进行的这段组句的按键账；没在组句为 `None`。
    pub trace: Option<CompositionTrace>,

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

/// 检查更新的结果文件名，与学习数据同目录（见 `qingjian-update::UpdateState`）。
const UPDATE_STATE_FILE: &str = "update.json";

/// 词汇记录文件名，与学习数据同目录（一个译词一行，见 `qingjian-learning::VocabularyBook`）。
const VOCABULARY_FILE: &str = "user-vocab.tsv";

/// 可能打进包里的释义表语言，按这个顺序在设置里列出；文件不存在的不列。
const GLOSSARY_LANGUAGES: [Language; 5] = [
    Language::English,
    Language::Japanese,
    Language::Spanish,
    Language::Vietnamese,
    Language::German,
];

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
