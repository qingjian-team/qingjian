mod appearance;
mod apps;
mod aux_code;
mod candidate_renderer;
mod code_rules;
mod commit_key;
mod dictionaries;
mod emoji_count;
mod font_size;
mod general;
mod highlight_keys;
mod key_combo;
mod key_hint_mode;
mod layout_mode;
mod legacy_traditional;
mod log_level;
mod model;
mod modifiers;
mod numpad_digit;
mod optional_key_combo;
mod page_keys;
mod phrases;
mod preedit_mode;
mod retired;
mod scheme;
mod shift_letter;
mod shortcut;
mod status_bar;
mod switch_key;
mod third_input_method;
mod transfer;
mod update;

use std::collections::BTreeMap;
use std::path::Path;

use qingjian_core::FuzzyRules;
use qingjian_predict::PredictConfig;
use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::error::ConfigError;

pub use appearance::Appearance;
pub use apps::{
    AppsConfig, DEFAULT_ENGLISH_CANDIDATES_ON, DEFAULT_ENGLISH_CANDIDATES_ON_LINUX,
    DEFAULT_ENGLISH_CANDIDATES_ON_MACOS, DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS,
};
pub use aux_code::AuxCodeConfig;
pub use candidate_renderer::CandidateRenderer;
pub use code_rules::CodeRules;
pub use commit_key::{CommitKey, CommitKeys, CommitRoute};
pub use dictionaries::{DEFAULT_DOMAINS, DictionariesConfig};
pub use emoji_count::EmojiCount;
pub use font_size::FontSize;
pub use general::{
    DEFAULT_KEY_HINT_OPACITY, DEFAULT_PAGE_KEYS, DEFAULT_THEME, GeneralConfig,
    LEARNING_LANGUAGE_OFF, MAX_PAGE_SIZE, PAGE_KEY_OPTIONS, UI_LANGUAGE_DEFAULT,
    UI_LANGUAGE_SYSTEM,
};
pub use highlight_keys::HighlightKeys;
pub use key_combo::KeyCombo;
pub use key_hint_mode::KeyHintMode;
pub use layout_mode::LayoutMode;
pub use legacy_traditional::LegacyTraditional;
pub use log_level::LogLevel;
pub use model::{LocalModel, LocalModelConfig, ZhiyiTier};
pub use modifiers::Modifiers;
pub use numpad_digit::NumpadDigit;
pub use page_keys::PageKeys;
pub use phrases::{export_phrases, import_phrases};
pub use preedit_mode::PreeditMode;
pub use scheme::{Scheme, scheme_label, scheme_name};
pub use shift_letter::ShiftLetter;
pub use shortcut::ShortcutConfig;
pub use status_bar::StatusBarConfig;
pub use switch_key::{SwitchKey, SwitchKeys};
pub use third_input_method::ThirdInputMethod;
pub use transfer::{export_settings, import_settings};
pub use update::{UpdateChannel, UpdateConfig};

/// 用户配置文件（TOML）。所有平台同一份格式，缺省值全部在各分节的 `Default` 里。
///
/// 配置文件是唯一事实源：菜单、设置窗口、手改文件三个入口都只写这个文件，再由壳热加载。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 常规：学习语言、每页候选数、翻页键、外观。
    pub general: GeneralConfig,

    /// 自定义短语；保存和读取均检查位置冲突。读的时候写错的条目只跳过它自己（见 `phrases`）。
    #[serde(deserialize_with = "phrases::deserialize")]
    pub custom_phrases: Vec<qingjian_core::CustomPhrase>,

    /// 快捷键：前缀模式键（表达式 / 问字）与上屏译词的修饰键组合。
    pub shortcut: ShortcutConfig,

    /// 模糊音开关。
    pub fuzzy: FuzzyRules,

    /// 附加词库开关。
    pub dictionaries: DictionariesConfig,

    /// 辅码码表开关。
    pub aux_code: AuxCodeConfig,

    /// 按应用改行为（哪些应用里英文模式不给候选）。
    pub apps: AppsConfig,

    /// 云联想。
    pub predict: PredictConfig,

    /// 悬浮状态条（桌面上常驻、可拖动的中 / 英浮窗）。
    pub status_bar: StatusBarConfig,

    /// 本地整句模型。
    pub model: LocalModelConfig,

    /// 检查更新。
    pub update: UpdateConfig,

    /// 各形码方案的按键规则（`[code.<id>]`），按方案 id；没写的方案按它自己的 `scheme.toml`。
    pub code: BTreeMap<String, CodeRules>,
}

/// 模板的 `[apps]` 一节（Linux）：应用按 fcitx5 认到的名字（X11 是 WM_CLASS，Wayland 是 app_id）。
/// 白名单要与 [`DEFAULT_ENGLISH_CANDIDATES_ON`] 一致，测试 `template_parses_to_defaults` 会核对。
#[cfg(not(any(windows, target_os = "macos")))]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按应用改行为，条目是 fcitx5 认到的应用名（X11 是 WM_CLASS，Wayland 是 app_id；`*` 结尾按前缀匹配，不区分大小写）
# 英文模式下给候选的应用（白名单）：浏览器、聊天、办公与笔记这类写成段文字的地方。没列的应用里英文字母原样交给应用，
# 免得挡住终端 / 编辑器自己的补全，或吞掉设计、3D 软件的单键快捷键。写成 ["*"] 就处处都给，再用 english_candidates_off 排除个别应用
english_candidates_on = [
  "firefox", "org.mozilla.firefox", "google-chrome", "chromium", "chromium-browser", "microsoft-edge", "brave-browser",
  "thunderbird", "org.mozilla.thunderbird", "libreoffice*", "soffice", "wps", "wpp",
  "gedit", "org.gnome.gedit", "org.gnome.TextEditor", "kate", "org.kde.kate", "kwrite", "mousepad",
  "wechat", "telegram-desktop", "org.telegram.desktop", "discord", "slack", "obsidian", "typora",
]
"#
    };
}

/// 模板的 `[apps]` 一节（macOS）：应用按 bundle identifier 认。白名单要与 [`DEFAULT_ENGLISH_CANDIDATES_ON`] 一致，
/// 测试 `template_parses_to_defaults` 会核对。用宏而不是常量，是因为 `concat!` 只收字面量。
#[cfg(target_os = "macos")]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按应用改行为，条目是 bundle identifier（`*` 结尾按前缀匹配）。切到一个应用时运行日志里会记下它的 bundle identifier
# 英文模式（Caps Lock）下给候选的应用（白名单）：浏览器、聊天、办公与笔记这类写成段文字的地方。没列的应用里英文字母原样交给应用，
# 免得挡住终端 / 编辑器自己的补全，或吞掉设计、3D 软件的单键快捷键。写成 ["*"] 就处处都给，再用 english_candidates_off 排除个别应用
english_candidates_on = [
  "com.apple.Safari", "com.google.Chrome*", "org.mozilla.firefox", "com.microsoft.edgemac*", "company.thebrowser.Browser", "com.brave.Browser*",
  "com.apple.TextEdit", "com.apple.Notes", "com.apple.mail", "com.apple.MobileSMS", "com.apple.iWork.Pages", "com.apple.iWork.Keynote",
  "com.microsoft.Word", "com.microsoft.Powerpoint", "com.microsoft.Outlook", "com.microsoft.onenote.mac", "com.microsoft.teams2", "com.kingsoft.wpsoffice.mac",
  "com.tencent.xinWeChat", "com.tencent.qq", "com.tencent.WeWorkMac", "com.alibaba.DingTalkMac", "com.electron.lark",
  "ru.keepcoder.Telegram", "com.tdesktop.Telegram", "com.hnc.Discord", "com.tinyspeck.slackmacgap", "com.vng.zalo", "com.anthropic.claudefordesktop", "md.obsidian", "notion.id", "abnerworks.Typora",
]
"#
    };
}

/// 模板的 `[apps]` 一节（Windows）：应用按宿主进程的 exe 文件名认。白名单要与 [`DEFAULT_ENGLISH_CANDIDATES_ON`] 一致。
#[cfg(windows)]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按应用改行为，条目是应用进程的 exe 文件名（`*` 结尾按前缀匹配）。Server 开着 debug 日志时每开一个会话会把 exe 名记进日志
# 英文模式下给候选的应用（白名单）：浏览器、聊天、办公与笔记这类写成段文字的地方。没列的应用里英文字母原样交给应用，
# 免得挡住终端 / 编辑器自己的补全，或吞掉 Photoshop 这类软件的单键快捷键（W / H / V）。写成 ["*"] 就处处都给，再用 english_candidates_off 排除个别应用
english_candidates_on = [
  "chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "vivaldi.exe", "notepad.exe",
  "WINWORD.EXE", "POWERPNT.EXE", "OUTLOOK.EXE", "ONENOTE.EXE", "ms-teams.exe", "wps.exe", "wpp.exe",
  "Weixin.exe", "WeChat.exe", "QQ.exe", "WXWork.exe", "DingTalk.exe", "Feishu.exe", "Lark.exe",
  "Telegram.exe", "Discord.exe", "slack.exe", "thunderbird.exe", "Obsidian.exe", "Notion.exe", "Typora.exe",
]
"#
    };
}

/// 模板 `[shortcut]` 一节里的修饰键组合（macOS 命名）。缺省值两个平台一样，只是写法与注释按平台的键名。
#[cfg(target_os = "macos")]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 中 / 英模式切换键，可多选：shift 单击 / control 单击 / ctrl+alt+space 组合键；[] 不用键切换。
# macOS 只认 shift：写成 ["shift"] 就是单击 Shift 切中英（此时 Caps Lock 只管大小写）；
# 缺省 [] —— 用 Caps Lock 切换（系统级，不受本项影响）
switch_mode = []
# 数字键配这些修饰键上屏候选的译词：translation 第一个译词，translation_second 第二个（候选右侧有两个译词时）
# 任意修饰键组合（option / shift / control / command 用 + 连），偏好设置里点按钮录制；别用 control+数字（系统切桌面）和 command+数字（应用切标签页）
translation = "option"
translation_second = "shift+option"
# 把应用里选中的文字译成学习语言（要开着云服务）：译文先出现在候选窗口，回车替换选中的文字，Esc 保留原文
# 修饰键 + 一个字母或数字，任意组合；避开 ⌘T 这类应用常用键
translate_selection = "control+option+t"
"#
    };
}

/// Linux 与 FreeBSD 默认避开桌面常用的 Ctrl + Alt + T（打开终端）。
#[cfg(not(any(windows, target_os = "macos")))]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 中 / 英模式切换键，可多选：shift 单击（缺省）/ control 单击 / ctrl+alt+space 组合键；[] 不用键切换，单击 Shift 原样交给应用
switch_mode = ["shift"]
# 数字键配这些修饰键上屏候选的译词：translation 第一个译词，translation_second 第二个
translation = "alt"
translation_second = "shift+alt"
# 把应用里选中的文字译成学习语言（要开着云服务）；Ctrl + Alt + T 常被桌面占用来打开终端
translate_selection = "alt+shift+t"
"#
    };
}

/// 模板 `[shortcut]` 一节里的修饰键组合（Windows 键名：alt / ctrl / win，读回来与 macOS 的 option / control / command 等价）。
#[cfg(windows)]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 中 / 英模式切换键，可多选：shift 单击（缺省，与微软拼音一致）/ control 单击 / ctrl+alt+space 组合键；[] 不用键切换，只剩按钮。
# 不提供 Ctrl + Space：中文 Windows 把它绑成系统的「输入法/非输入法切换」，系统先截走
switch_mode = ["shift"]
# 数字键配这些修饰键上屏候选的译词：translation 第一个译词，translation_second 第二个（候选右侧有两个译词时）
# 任意修饰键组合（alt / shift / ctrl / win 用 + 连）。Alt+数字会被 Windows 当菜单快捷键截走，缺省用 Ctrl；组句时才拦，不打字时照常放行给应用
translation = "ctrl"
translation_second = "shift+ctrl"
# 把应用里选中的文字译成学习语言（要开着云服务）：译文先出现在候选窗口，回车替换选中的文字，Esc 保留原文
translate_selection = "ctrl+alt+t"
# Windows 组句中按需调用 AI
predict = "ctrl+alt+j"
"#
    };
}

/// 首次运行写出的模板：默认值全部列出并注释，用户改一处即可。`[shortcut]` 的修饰键与 `[apps]` 分平台，
/// 见 [`template_shortcut_keys!`] / [`template_apps!`]。
pub const TEMPLATE: &str = concat!(
    r#"# 青简输入法配置。保存后自动生效；也可以在菜单栏的输入法菜单里改。

[general]
# 学习语言（en 英语 / ja 日语 / es 西班牙语 / de 德语 / off 不显示译文）：候选旁显示哪种语言的译文，要有对应的释义表才生效
learning_language = "en"
# 每页候选数（1–9）
page_size = 9
# 翻页键对：前一个上一页、后一个下一页。可选 "[]"、",." 或 "-="
# 选 ",." 时组句中逗号句号用于翻页而不是上屏加标点；选 "-=" 时中文组句中的 - 用于翻页，不再进入英文直输段
page_keys = "[]"
# 候选窗口外观：system 跟随系统 / light 浅色 / dark 深色
appearance = "system"
# 候选窗口主题（主题 id）：qingjian 青简绿 / system-blue 系统蓝 / wechat 微信绿 / sakura 樱花 / dafeiyu 大肥鱼 / miku 初音未来 / bocchi 波奇酱。只对青简渲染器生效
theme = "qingjian"
# 候选窗口排布：vertical 竖排 / horizontal 横排（横排只给高亮候选显示译文）
layout = "vertical"
# 横排时 ↑ / ↓ 把单行展开成 6 行矩阵并换行（一行一页候选），← / → 改为在候选之间移动（拼音光标用 ⌥←/→、⌘←/→），
# Esc 第一下先收回单行。缺省 false：横排下 ↑ / ↓ 逐个移动高亮、← / → 移动拼音光标，与以前一样。只有 macOS 用
horizontal_grid = false
# 横排时翻页键（主翻页键、额外翻页键、PageUp / PageDown、Tab）也像 ↓ 一样展开成矩阵并换行，展开后一次翻一屏。
# 缺省 false：翻页键照旧整页翻。macOS 与 Windows 用
grid_page_keys = false
# 候选窗口由谁绘制：qingjian 青简渲染器（各平台一致，主题走它）/ system 系统原生绘制（渲染器有问题时的退路）
renderer = "qingjian"
# 候选窗口字体（字族名，如 "LXGW WenKai"）；空为系统字体。只对青简渲染器生效，没装这个字体时自动回到系统字体
font = ""
# 候选字、译文的字号（点），盖过主题；0 用主题的。只对青简渲染器生效
candidate_font_size = 0
annotation_font_size = 0
# 候选窗口的过渡动画（高亮滑动、主题里的循环动画）；false 时直接跳到位，与系统「减弱动态效果」一样
animations = true
# 组句中的拼音显示在哪：both 行内和候选窗口 / inline 只在行内 / window 只在候选窗口（应用里不放 marked text）
preedit = "both"
# 英文模式（Caps Lock 亮着）是否给英文候选：Tab 或方向键选词，空格、回车、标点仍原样上屏敲的字母；false 就是纯直通
english_candidates = true

# 繁体输出：off 简体 / taiwan 台湾字形 / hongkong 香港字形 / standard 通用字形；三档都只换字形不换用语（软件 → 軟件）
# 候选窗口里看到的就是转换后的字；词库与学习数据始终是简体，改回 off 之前学到的词照样在。旧版的 traditional = true 按台湾字形读
traditional_variant = "off"
# 中文模式下整段输入是英文词时（hello / key）是否让中文候选排第一、英文词第二；缺省 false：拼音不像话的输入英文词排第一
chinese_first = false
# 拼写纠错：拼音敲错时给纠正后的候选（nihooma → 你好吗）；敲的拼音照原样读得通时纠正只当备选。false 时只按敲的原样读
spelling_correction = true
# 中文模式下整段是英文词或英文词的开头时给英文候选与补全（hello、compa → company）；false 时中文模式只出中文，英文模式不受影响
english_in_chinese = true
# 中文模式下英文候选（精确词 + 补全）最多几个，1–4；1 时只留最靠前的一条
english_max_in_chinese = 4
# 有中文候选时英文最高排到第几位，1–9；1 为不限制，没有中文候选时英文照旧第一
english_min_position = 1
# 候选后面一次最多配几个 emoji（kaixin → 开心 😄），0–3；0 时候选里只有字词
emoji = 3
# 候选后面一次最多配几个颜文字（ku → 哭 (T_T)），0–3
kaomoji = 1
# 候选后面一次最多配几个符号（yuedengyu → 约等于 ≈，alpha → α），0–3
symbols = 1
# 中西文之间自动补空格：汉字与半角字母 / 数字相邻时补一个空格，「用Go写了30行」上屏成「用 Go 写了 30 行」。缺省关；只有 macOS 用
auto_space = false
# 中文模式下按住 Shift 敲的字母：passthrough 拼音原样上屏、字母交给应用（缺省，与以前一致）/ compose 收进组句
# 缓冲区参与匹配，这样 Cpan 与 cpan 一样能出「C盘」。英文模式与英文直输段（no-Way）不受影响
shift_letter = "passthrough"
# Windows 中文组句中的小键盘数字：select 选择候选（缺省）/ direct 直接输入数字；主键盘数字不受影响
numpad_digit = "select"
# 内置英文模式：开着时单击切换键（[shortcut] switch_mode）或 Caps Lock 亮着进英文模式
# 关掉后青简保持中文模式，切换键与语言栏按钮都不再切过去；要打英文请用系统快捷键（Win+Space）切到别的输入法。Windows 与 Linux 用，macOS 的中英切换是 Caps Lock
english_mode = true
# 连按两次 Shift 切换到的第三输入法：off 不启用 / vietnamese 越南语。当前版本先保存设置，输入法实现完成后生效
double_shift_input = "off"
# 初始英文：Windows 是青简启动时，按应用记忆时也用于没有记录的应用；Linux 是新打开的应用和窗口
default_english = false
# Windows：按应用 exe 名记住中英状态，重启青简后重新使用初始模式
remember_mode_per_app = false
# 中文模式下（没在组句时）敲的标点转全角：, . ? ! : ; ( ) 等，数字后面的 . 保持半角。Windows 上悬浮状态条的「，。」格可以点着切；macOS 在偏好设置中选择默认中文标点模式
full_width_punctuation = true
# 英文模式下的同一件事，中英各记一份，状态条切的是当前模式那份；只有 Windows 用
english_full_width_punctuation = false
# 组句中敲会转全角的标点（, . ? ! 等，翻页键除外）先把高亮候选上屏、再补该标点（nihao, 一气打完「你好，」）；
# 缺省 false：标点进英文直输段，与以前一致
punct_commits = false
# 辅码触发键：拼音打完之后敲它进辅码态，之后敲的字母按码表缩小候选范围；缺省是分号
# 单个可见字符，字母、数字与翻页键不能当触发键；微软 / 搜狗双拼里分号先当 ing 的韵母键
aux_code_key = ";"
# 候选上是否显示码（方括号紧跟在候选词后面，如「鹤[rbm] crane」）。缺省关
aux_code_show = false
# 候选窗口顶部的拼音行按高亮候选显示带调拼音（gān'gà），应用里的拼音不变；macOS、Windows 用。缺省关
show_tones = false
# 码段删空后是否留在辅码状态：true 删空后 ; 仍在、候选全部回来，再按一次退格才退出辅码；false 删空即回拼音状态
aux_code_keep_empty = true
# 拼音方案：留空或 pinyin 为全拼 / xiaohe 小鹤双拼 / ziranma 自然码 / microsoft 微软双拼 / sogou 搜狗双拼 / abc 智能ABC / xiaolang 小浪双拼 / shoudao 首道双拼 / jiajia 拼音加加 / ziguang 紫光双拼 /
# zhuyin 大千注音 / none 关（只用形码，见下面的 code）。
# 双拼与注音下 v / u / i 都是按键，表达式模式没有入口，问字只能靠 question_mark 打开后用 ? 进；微软、搜狗、紫光方案的 ; 键是 ing
scheme = ""
# 双拼方案下 preedit 显示原始按键（如 ljse）还是展开成全拼（lan'se）；缺省 false（展开成全拼）
shuangpin_raw_preedit = false
# 双拼方案下候选窗口的拼音行显示原始按键还是全拼，与上一项各管各的；缺省 false（全拼）；macOS、Windows 用
shuangpin_raw_window = false
# 双拼方案下同时接受全拼：开着时打拼音也用全拼，同一段输入里两种打法可以混用（一边双拼一边全拼的人共用一份配置）
shuangpin_full_pinyin = false
# 一段拼音分几次选完时，先选的词留在输入框里、整段选完才一起上屏，退格先撤回最后一次选择；false 为选了就上屏
hold_selections = true
# 形码方案：留空或 off 为关，wubi86 为五笔（86 版），也可以写用户数据目录 schemes/ 下自己放的方案目录名。
# **与上面的拼音方案同时开着就是混输**——两边都出候选，编码打全的形码词在前、其次拼音（打不出的字直接打拼音）；
# 候选旁的译文、生词记录与学习照常。只用形码的话把 scheme 写成 none，这时四码唯一自动上屏、满码再敲字母顶字。
code = ""
# 日志级别：info 缺省 / debug 详细（会记录敲的拼音与上屏的文字，配合作者排查问题时再开）。日志在 ~/Library/Logs/Qingjian/
log_level = "info"
# 输入日志：每次上屏记一行到数据目录的 input-log.jsonl（敲的键、看到的候选、选了什么），只写在这台电脑上，不上传；
# 用来离线评测排序和训练个人模型。false 不记；「高级」页可以清空
input_log = true
# 学习输入习惯：按你的选择调整候选顺序、记新词与敲错纠正。false 不再学，已学的仍参与排序；学习数据在数据目录，删掉文件即清空
learning = true
# 把系统设置「键盘 → 文本替换」里的条目当自定义短语：输入码（小写字母）敲全后出现，排在同权重的自定义短语之后；只有 macOS 用
system_text_replacements = true

# 自定义短语示例：取消下面各行注释后启用；同一个输入码下按权重从高到低排在普通候选前面。
# [[custom_phrases]]
# code = "ww"       # 输入码：1–32 个小写英文字母
# text = "；"       # 原样上屏的文本，可包含空格与换行；{year} {month} {day} {hour} {minute} {second} {weekday} 换成当前时间，{{ }} 是花括号本身
# weight = 10       # 权重：1–999，可以相同，相同的按文件里的先后
# enabled = true    # 是否启用

[shortcut]
# Windows：切换当前模式的全 / 半角标点；空串禁用，可设为 "ctrl+." 或其他修饰键加字母、数字、句号
toggle_punctuation = ""
# Windows：在简体与上次用的繁体字形之间切换；空串禁用，可设为 "ctrl+shift+f" 或其他修饰键加字母、数字
toggle_traditional = ""
# Windows：主翻页键之外同时启用的键对，可多选，如 ["[]", ",.", "-="]
extra_page_keys = []
# 前缀模式键，只能是 v / u / i 之一且互不相同（这三个字母不是任何拼音音节的开头）
# 表达式模式：v1+2 出 3，v123 出中文数字
expression = "v"
# 问字模式：usangemu 问「三个木」（云端答），u4e00 出码点对应的字符（本地答）
question = "u"
# 没在组句时敲 ? 是否也进问字模式（中英文模式都行，后面跟字母才是问题，跟别的键还原成问号）；false 的话问号就是问号
question_mark = false
# 候选高亮由哪对方向键移动：updown 用 ↑ / ↓ 移高亮、← / → 移拼音光标（缺省）；leftright 改用 ← / → 移高亮
# leftright 时两键把「拼音行 + 候选栏」当成一条从左到右的序列走：→ 到拼音光标末位溢出成选词，← 到候选首位回拼音光标；↑ / ↓ 不做事。只有 Windows 用
highlight_keys = "updown"
# 额外用 Ctrl + F/B/N/P/A/E 导航候选（macOS）：竖排 P/N 移候选、B/F 翻页，横排相反；A/E 跳到当前页首个/最后一个真实候选，跳过空位；默认关
extra_candidate_navigation = false
# 组句时上屏高亮候选的键与原样上屏敲的字母的键：space / enter，都可写成 shift+space、shift+enter，或一个符号键如 ";"
# 不能用 Ctrl / Alt / Win 组合、字母数字、翻页键；两项不能相同。空格或回车让出去后又没配给另一项，组句时按它不做事。Windows、Linux 用
commit_candidate = "space"
commit_raw = "enter"
"#,
    template_shortcut_keys!(),
    r#"
[fuzzy]
# 模糊音：开了之后敲 zi 也出 zhi 的字、敲 lan 也出 nan 的字。默认全关，按需打开。
z_zh = false
c_ch = false
s_sh = false
n_l = false
f_h = false
l_r = false
an_ang = false
en_eng = false
in_ing = false

"#,
    template_apps!(),
    r#"
[dictionaries]
# 随包的领域词库（法律 / 医学 / 地名 / 成语 / 诗词 / IT / 财经 / 饮食 / 动物 / 汽车 / 历史人物），列在这里的才加载；
# 名字是文件名：animals automotive finance food historical_figures idioms it_computing law medicine places poetry_lines。
# 偏好设置「词库」页可以勾选
domains = ["idioms"]
# 自己导入的词库：放在配置同目录 dicts/ 下的 .qj 文件都会加载，这里列出要关掉的（文件名，不含扩展名）
disabled = []

[aux_code]
# 辅码总开关：false 时整条辅码线关（; 完全保持原生行为，候选也不挂码）；
# true 且有可用码表（随包或 codes/ 下有 .qj）才生效
enabled = false
# 辅码码表：放在配置同目录 codes/ 下的 .qj 文件都会加载，这里列出要关掉的（文件名，不含扩展名）
# 随包的笔画表也可以在这里关掉；码表由「辅码」设置页导入，或放好文件后在这里管
disabled = []

[model]
# 本地模型：在本机给整句候选重新排序或直接造句，全程离线；停顿后几十毫秒生效。缺省关，只用词库统计
enabled = false
# 用哪个：tongbian 含章·通变（按拼音造句）/ zhiyi 知意（按前文挑首选）；先在设置的「实验功能」里下载（Linux 随包带通变），没下载就不用模型
name = "tongbian"
# 知意的档位：saver 省电 / balanced 均衡 / accurate 准确（比的候选更多、看的前文更长，也更费电）
zhiyi_tier = "balanced"
# 应用读不到光标前的文字时，拿这个应用、这个输入框里刚打的字当前文；多少秒没打字就不算（0 表示一直算）
context_memory_seconds = 60

[predict]
# 云联想：把光标附近的文本发到下面的接口，让模型补全整句 / 联想下文。默认关闭。
# 开启后菜单栏的「中 / 英」旁会带一个云朵标识；Secure Input（密码框）里绝不发送。
enabled = false
# OpenAI 兼容接口地址与模型名（DeepSeek 默认值）
base_url = "https://api.deepseek.com"
model = "deepseek-flash"
# 推理强度（reasoning_effort）：none 关掉模型的思考，联想要快；留空则不发这个参数
reasoning_effort = "none"
# 密钥：填在这里，或留空并设置 api_key_env 指定的环境变量（偏好设置里填的密钥写进配置同目录的 .env）
# api_key = ""
api_key_env = "QINGJIAN_API_KEY"
# 单次请求超时（毫秒）、停止敲键多久后才发请求（毫秒）
timeout_ms = 5000
debounce_ms = 300
# Windows 自动联想开关；关闭后在组句中按 Ctrl+Alt+J 手动请求，快捷键在 [shortcut] predict 中设置
automatic = true
# 两次网络请求起始之间的最短间隔，失败也计入；缓存命中不等待
min_interval_ms = 3000
# 光标前 / 后最多发多少个字符——这是发往云端的上下文上限
lookback = 64
lookahead = 32
# 云端词到了补进候选窗口第一页末尾几格（比如 2 就是 8、9 两格），前面的本地候选不动；0 表示不要云端词
slots = 2
# 组句中除了词候选还要不要整句补全（preedit 右侧，Tab 接受）
sentence = true
# 本地整句候选右侧是否显示服务返回的译文；仅发送该候选文本，不发送光标前后上下文
sentence_translation = false
# 上屏的整句（含分几次打完的一句）翻成学习语言，记进生词本的时间线；只发那一句，不发光标前后的文字
vocabulary_translation = true

[status_bar]
# 桌面上常驻、可拖动的悬浮状态条（Windows）：「中 / 英」格点一下切换模式（开着双拼时还显示方案名）、「，。」格切全角 / 半角标点、齿轮打开设置。
# 只在当前输入法是青简时显示；与任务栏的中 / 英指示器并存
# 默认关；开着时可以拖到任意位置，拖到哪下次还在哪（拖动结束时把位置写进下面的 x / y，不用手填）
enabled = false
# 记住的屏幕位置（物理像素，拖动后自动写入）；留空则首次出现在屏幕右下角
# x = 0
# y = 0
# 大小（百分比，60–150）；不跟候选字号走
size = 100

[update]
# 检查更新：每天向官网（qingjian.app）读一次版本索引，有新版在菜单与设置的「关于」页提示；请求不带任何标识，不自动下载安装
check = true
# 渠道：stable 只看正式版；beta 还会提示测试版（alpha / beta / rc）
channel = "stable"
"#
);

impl Config {
    /// 用户给形码方案 `id` 改的按键规则；没改过时全是 `None`。
    pub fn code_rules(&self, id: &str) -> CodeRules {
        self.code.get(id).copied().unwrap_or_default()
    }

    /// 保存自定义短语列表，冲突时不修改文件。
    pub fn set_custom_phrases(
        path: &Path,
        phrases: &[qingjian_core::CustomPhrase],
    ) -> Result<(), String> {
        qingjian_core::custom_phrase::validate_phrases(phrases)?;
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(e) => return Err(e.to_string()),
        };
        let mut document: DocumentMut = source.parse::<DocumentMut>().map_err(|e| e.to_string())?;
        let existing = document
            .get("custom_phrases")
            .and_then(toml_edit::Item::as_array_of_tables)
            .cloned()
            .unwrap_or_default();
        // 读配置时跳过的条目界面上看不到，保存时原样留在文件里，等用户自己改
        let checked = phrases::check(existing.iter().map(|table| {
            toml::from_str::<qingjian_core::CustomPhrase>(&table.to_string())
                .map_err(|e| e.message().to_owned())
        }));
        let mut old = toml_edit::ArrayOfTables::new();
        let mut skipped = Vec::new();
        for (table, item) in existing.iter().zip(&checked) {
            if item.is_ok() {
                old.push(table.clone());
            } else {
                skipped.push(table.clone());
            }
        }
        let mut used = std::collections::BTreeSet::new();
        // 先按输入码和文本匹配，重排或删除时注释跟随原规则。
        let mut matches: Vec<_> = phrases
            .iter()
            .map(|p| {
                let found = old
                    .iter()
                    .enumerate()
                    .find(|(_, t)| {
                        t.get("code").and_then(toml_edit::Item::as_str) == Some(p.code.as_str())
                            && t.get("text").and_then(toml_edit::Item::as_str)
                                == Some(p.text.as_str())
                    })
                    .map(|(i, _)| i);
                if let Some(i) = found {
                    used.insert(i);
                }
                found
            })
            .collect();
        // 等长列表中修改了输入码或文本的条目，沿用其未被占用的原表。
        if old.len() == phrases.len() {
            for (i, matched) in matches.iter_mut().enumerate() {
                if matched.is_none() && used.insert(i) {
                    *matched = Some(i);
                }
            }
        }
        let mut tables = toml_edit::ArrayOfTables::new();
        for (p, matched) in phrases.iter().zip(matches) {
            let mut t = matched
                .and_then(|i| old.get(i))
                .cloned()
                .unwrap_or_default();
            if t.contains_key("position") {
                t = phrases::position_to_weight(&t);
            }
            for (key, mut value) in [
                ("code", toml_edit::Value::from(p.code.as_str())),
                ("text", toml_edit::Value::from(p.text.as_str())),
                ("weight", toml_edit::Value::from(i64::from(p.weight))),
                ("enabled", toml_edit::Value::from(p.enabled)),
            ] {
                if let Some(previous) = t.get(key).and_then(toml_edit::Item::as_value) {
                    if previous.to_string() == value.to_string() {
                        continue;
                    }
                    *value.decor_mut() = previous.decor().clone();
                }
                t[key] = toml_edit::Item::Value(value);
            }
            // 序列化按文档位置排序；统一锚点后，同组条目使用本次列表顺序。
            t.set_position(old.iter().filter_map(toml_edit::Table::position).min());
            tables.push(t);
        }
        for table in skipped {
            tables.push(table);
        }
        document["custom_phrases"] = toml_edit::Item::ArrayOfTables(tables);
        write_file(path, &document.to_string()).map_err(|e| e.to_string())
    }

    /// 读配置。文件不存在按默认值；存在但解析失败报错，不要静默吞掉用户的笔误。
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let config: Self = toml::from_str(&source).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        // 配置或环境变量里的密钥登记给日志掩码；各进程都从这里加载配置，登记在这一处就够
        if let Some(key) = config.predict.resolve_api_key() {
            crate::logs::secrets::register(&key);
        }
        // 界面语言同理：输入法进程里的候选菜单、提示与设置程序都按这一项
        bamboo_slip_i18n::set_locale(config.general.ui_locale());
        Ok(config)
    }

    /// 原地改一个布尔键，见 [`Self::set_value`]。
    pub fn set_bool(path: &Path, section: &str, key: &str, value: bool) -> Result<(), ConfigError> {
        Self::set_value(path, section, key, value)
    }

    /// 原地改一个键（`[section] key = value`），其余内容、注释与顺序原样保留：
    /// 菜单和设置窗口落盘都走这里。文件不存在时从模板起步；文件有语法错误就报错不写，
    /// 不能替用户「修复」成丢了注释的文件。
    pub fn set_value(
        path: &Path,
        section: &str,
        key: &str,
        value: impl Into<toml_edit::Value>,
    ) -> Result<(), ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut document: DocumentMut = source.parse().map_err(|source| ConfigError::Edit {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        // 旧写法（外观写在 theme 里）要在改键之前认出来：写的正是 theme 时，新值会把它盖掉
        let legacy = (section == "general" && matches!(key, "appearance" | "theme"))
            .then(|| legacy_appearance(&document))
            .flatten();
        section_table(&mut document, section)[key] = toml_edit::value(value);
        if let Some(old) = legacy {
            migrate_legacy_theme(&mut document, old);
        }
        // 写临时文件再改名：输入法进程随时可能被杀，不能留半个配置文件
        write_file(path, &document.to_string())
    }

    /// 原地把一个键改成字符串数组（`[section] key = ["a", "b"]`），其余内容、注释与顺序原样保留。
    /// 设置界面改词库列表（`[dictionaries] domains` / `disabled`）走这里，[`Self::set_value`] 只能写标量。
    pub fn set_array<S: AsRef<str>>(
        path: &Path,
        section: &str,
        key: &str,
        values: &[S],
    ) -> Result<(), ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut document: DocumentMut = source.parse().map_err(|source| ConfigError::Edit {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        let mut array = toml_edit::Array::new();
        for value in values {
            array.push(value.as_ref());
        }
        section_table(&mut document, section)[key] = toml_edit::value(array);
        write_file(path, &document.to_string())
    }

    /// 文件不存在时写出模板（目录一并建），返回是否写了。
    pub fn write_template_if_missing(path: &Path) -> Result<bool, ConfigError> {
        if path.exists() {
            return Ok(false);
        }
        write_file(path, TEMPLATE)?;
        Ok(true)
    }
}

/// 模板里 `appearance` 那一行上方的注释，旧配置迁移时照抄（有测试保证与 [`TEMPLATE`] 一致）。
const APPEARANCE_COMMENT: &str = "# 候选窗口外观：system 跟随系统 / light 浅色 / dark 深色\n";

/// 模板里 `theme` 那一行上方的注释，同上。
const THEME_COMMENT: &str = "# 候选窗口主题（主题 id）：qingjian 青简绿 / system-blue 系统蓝 / wechat 微信绿 / sakura 樱花 / dafeiyu 大肥鱼 / miku 初音未来 / bocchi 波奇酱。只对青简渲染器生效\n";

/// 旧写法里 `theme` 写的外观；新写法（或没写）为 `None`。
fn legacy_appearance(document: &DocumentMut) -> Option<Appearance> {
    let value = document.get("general")?.get("theme")?.as_str()?;
    Appearance::from_key(value.trim())
}

/// 写 `appearance` 或 `theme` 时顺手迁移旧写法（2026-09-18 之前外观写在 `theme` 里，旧值是 `old`）：
/// 外观补写到 `appearance`（写的是 `theme` 时它还没有，不补就丢了），`theme` 还是外观词就换成内置主题，
/// 两行注释换成模板里的，免得手改配置的人看到「外观」注释下面是 `theme`。迁过之后 `theme` 不再是外观词，不会再迁。
fn migrate_legacy_theme(document: &mut DocumentMut, old: Appearance) {
    let Some(general) = document
        .get_mut("general")
        .and_then(toml_edit::Item::as_table_mut)
    else {
        return;
    };
    if !general.contains_key("appearance") {
        general["appearance"] = toml_edit::value(old.key());
    }
    let still_legacy = general
        .get("theme")
        .and_then(toml_edit::Item::as_str)
        .is_some_and(|value| Appearance::from_key(value.trim()).is_some());
    if still_legacy {
        general["theme"] = toml_edit::value(DEFAULT_THEME);
    }
    if let Some(mut key) = general.key_mut("theme") {
        key.leaf_decor_mut().set_prefix(THEME_COMMENT);
    }
    if let Some(mut key) = general.key_mut("appearance") {
        key.leaf_decor_mut().set_prefix(APPEARANCE_COMMENT);
    }
}

/// 原子写配置文件；数据目录还没有就先建（新账户第一次打开设置时输入法可能还没跑过）。
/// 取（没有就建）分节 `section`，`code.wubi86` 这样带点的是嵌套分节。
/// 一律建成标准表：否则 toml_edit 会写成顶层的行内表 `predict = { enabled = true }`；
/// 嵌套时外层是隐式的，文件里只出现 `[code.wubi86]`、不多出一个空的 `[code]`。
fn section_table<'a>(document: &'a mut DocumentMut, section: &str) -> &'a mut toml_edit::Table {
    let mut table = document.as_table_mut();
    let mut parts = section.split('.').peekable();
    while let Some(part) = parts.next() {
        if !table.get(part).is_some_and(|item| item.is_table()) {
            let mut child = toml_edit::Table::new();
            child.set_implicit(parts.peek().is_some());
            table.insert(part, toml_edit::Item::Table(child));
        }
        table = table[part].as_table_mut().expect("刚确认过是表");
    }
    table
}

fn write_file(path: &Path, text: &str) -> Result<(), ConfigError> {
    let write = || {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        qingjian_core::storage::write_atomic_str(path, text)
    };
    write().map_err(|source| ConfigError::Write {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests;
