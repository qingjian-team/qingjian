mod apps;
mod dictionaries;
mod general;
mod key_combo;
mod layout_mode;
mod log_level;
mod modifiers;
mod preedit_mode;
mod shortcut;
mod theme_mode;

use std::path::Path;

use qingjian_core::FuzzyRules;
use qingjian_predict::PredictConfig;
use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::error::ConfigError;

pub use apps::{
    AppsConfig, DEFAULT_ENGLISH_CANDIDATES_OFF, DEFAULT_ENGLISH_CANDIDATES_OFF_MACOS,
    DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS,
};
pub use dictionaries::{DEFAULT_DOMAINS, DictionariesConfig};
pub use general::{DEFAULT_PAGE_KEYS, GeneralConfig, MAX_PAGE_SIZE, PAGE_KEY_OPTIONS};
pub use key_combo::KeyCombo;
pub use layout_mode::LayoutMode;
pub use log_level::LogLevel;
pub use modifiers::Modifiers;
pub use preedit_mode::PreeditMode;
pub use shortcut::ShortcutConfig;
pub use theme_mode::ThemeMode;

/// 用户配置文件（TOML）。所有平台同一份格式，缺省值全部在各分节的 `Default` 里。
///
/// 配置文件是唯一事实源：菜单、设置窗口、手改文件三个入口都只写这个文件，再由壳热加载。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 常规：学习语言、每页候选数、翻页键、外观。
    pub general: GeneralConfig,

    /// 快捷键：前缀模式键（表达式 / 问字）与上屏译词的修饰键组合。
    pub shortcut: ShortcutConfig,

    /// 模糊音开关。
    pub fuzzy: FuzzyRules,

    /// 附加词库开关。
    pub dictionaries: DictionariesConfig,

    /// 按应用改行为（哪些应用里英文模式不给候选）。
    pub apps: AppsConfig,

    /// 云联想。
    pub predict: PredictConfig,
}

/// 模板的 `[apps]` 一节（macOS）：应用按 bundle identifier 认。名单要与 [`DEFAULT_ENGLISH_CANDIDATES_OFF`] 一致，
/// 测试 `template_parses_to_defaults` 会核对。用宏而不是常量，是因为 `concat!` 只收字面量。
#[cfg(not(windows))]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按应用改行为，条目是 bundle identifier（`*` 结尾按前缀匹配）。开着「详细日志」时切到一个应用会把它的 bundle identifier 记进日志
# 英文模式（Caps Lock）下不给候选的应用：终端与代码编辑器里候选窗口会挡住应用自己的补全，vim 里 Tab 和方向键也另有含义。设成 [] 就处处都给
english_candidates_off = [
  "com.apple.Terminal", "com.googlecode.iterm2", "dev.warp.Warp-Stable", "com.mitchellh.ghostty", "io.alacritty", "net.kovidgoyal.kitty",
  "com.microsoft.VSCode", "com.todesktop.230313mzl4w4u92", "dev.zed.Zed", "com.jetbrains.*", "org.vim.MacVim", "com.sublimetext.*",
  "com.apple.dt.Xcode", "com.neovide.neovide",
]
"#
    };
}

/// 模板的 `[apps]` 一节（Windows）：应用按宿主进程的 exe 文件名认。名单要与 [`DEFAULT_ENGLISH_CANDIDATES_OFF`] 一致。
#[cfg(windows)]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按应用改行为，条目是应用进程的 exe 文件名（`*` 结尾按前缀匹配）。Server 开着 debug 日志时每开一个会话会把 exe 名记进日志
# 英文模式（Caps Lock）下不给候选的应用：终端与代码编辑器里候选窗口会挡住应用自己的补全，vim 里 Tab 和方向键也另有含义。设成 [] 就处处都给
# 经典控制台（cmd / PowerShell）的窗口属于 conhost.exe，Windows Terminal 是 WindowsTerminal.exe
english_candidates_off = [
  "conhost.exe", "WindowsTerminal.exe", "alacritty.exe", "wezterm-gui.exe", "mintty.exe",
  "Code.exe", "Code - Insiders.exe", "Cursor.exe", "zed.exe",
  "idea64.exe", "pycharm64.exe", "clion64.exe", "rustrover64.exe", "goland64.exe", "rider64.exe", "webstorm64.exe", "phpstorm64.exe", "datagrip64.exe",
  "devenv.exe", "sublime_text.exe", "notepad++.exe", "gvim.exe", "neovide.exe",
]
"#
    };
}

/// 模板 `[shortcut]` 一节里的修饰键组合（macOS 命名）。缺省值两个平台一样，只是写法与注释按平台的键名。
#[cfg(not(windows))]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 数字键配这些修饰键上屏候选的译词：translation 第一个译词，translation_second 第二个（候选右侧有两个译词时）
# 任意修饰键组合（option / shift / control / command 用 + 连），偏好设置里点按钮录制；别用 control+数字（系统切桌面）和 command+数字（应用切标签页）
translation = "option"
translation_second = "shift+option"
# 把应用里选中的文字译成学习语言（要开着云服务）：译文先出现在候选窗口，回车替换选中的文字，Esc 保留原文
# 修饰键 + 一个字母或数字，任意组合；避开 ⌘T 这类应用常用键
translate_selection = "control+option+t"
# 数字键配这些修饰键删掉候选：用户词（云端选过的、自动造的）整个删掉，词库里的词清掉对它的学习记录。组句中要打感叹号先把词上屏
delete_candidate = "shift"
"#
    };
}

/// 模板 `[shortcut]` 一节里的修饰键组合（Windows 键名：alt / ctrl / win，读回来与 macOS 的 option / control / command 等价）。
#[cfg(windows)]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 数字键配这些修饰键上屏候选的译词：translation 第一个译词，translation_second 第二个（候选右侧有两个译词时）
# 任意修饰键组合（alt / shift / ctrl / win 用 + 连）。Alt+数字会被 Windows 当菜单快捷键截走，缺省用 Ctrl；组句时才拦，不打字时照常放行给应用
translation = "ctrl"
translation_second = "shift+ctrl"
# 把应用里选中的文字译成学习语言（要开着云服务）：Windows 上还没接
translate_selection = "ctrl+alt+t"
# 数字键配这些修饰键删掉候选：用户词（云端选过的、自动造的）整个删掉，词库里的词清掉对它的学习记录。组句中要打感叹号先把词上屏
delete_candidate = "shift"
"#
    };
}

/// 首次运行写出的模板：默认值全部列出并注释，用户改一处即可。`[shortcut]` 的修饰键与 `[apps]` 分平台，
/// 见 [`template_shortcut_keys!`] / [`template_apps!`]。
pub const TEMPLATE: &str = concat!(
    r#"# 青简输入法配置。保存后自动生效；也可以在菜单栏的输入法菜单里改。

[general]
# 学习语言（en 英语 / ja 日语）：候选旁显示哪种语言的译文，要有对应的释义表才生效
learning_language = "en"
# 每页候选数（1–9）
page_size = 9
# 翻页键对：前一个上一页、后一个下一页。可选 "[]" 或 ",."；选 ",." 的话组句中敲逗号句号是翻页而不是上屏加标点
page_keys = "[]"
# 候选窗口外观：system 跟随系统 / light 浅色 / dark 深色
theme = "system"
# 候选窗口排布：vertical 竖排 / horizontal 横排（横排只给高亮候选显示译文）
layout = "vertical"
# 组句中的拼音显示在哪：both 行内和候选窗口 / inline 只在行内 / window 只在候选窗口（应用里不放 marked text）
preedit = "both"
# 英文模式（Caps Lock 亮着）是否给英文候选：Tab 或方向键选词，空格、回车、标点仍原样上屏敲的字母；false 就是纯直通
english_candidates = true
# 双拼方案：留空为全拼；xiaohe 小鹤 / ziranma 自然码 / microsoft 微软 / sogou 搜狗
# 开着时 v / u / i 都是音节键，表达式与问字模式只能用 ? 开头进；微软、搜狗方案的 ; 键是 ing
shuangpin = ""
# 日志级别：info 缺省 / debug 详细（会记录敲的拼音与上屏的文字，配合作者排查问题时再开）。日志在 ~/Library/Logs/Qingjian/
log_level = "info"
# 输入日志：每次上屏记一行到数据目录的 input-log.jsonl（敲的键、看到的候选、选了什么），只写在这台电脑上，不上传；
# 用来离线评测排序和训练个人模型。false 不记；「高级」页可以清空
input_log = true

[shortcut]
# 前缀模式键，只能是 v / u / i 之一且互不相同（这三个字母不是任何拼音音节的开头）
# 表达式模式：v1+2 出 3，v123 出中文数字
expression = "v"
# 问字模式：usangemu 问「三个木」（云端答），u4e00 出码点对应的字符（本地答）。? 开头永远也是问字
question = "u"
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

[predict]
# 云联想：把光标附近的文本发到下面的接口，让模型补全整句 / 联想下文。默认关闭。
# 开启后菜单栏的「中 / 英」旁会带一个云朵标识；Secure Input（密码框）里绝不发送。
enabled = false
# OpenAI 兼容接口地址与模型名（DeepSeek 默认值）
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
# 推理强度（reasoning_effort）：none 关掉模型的思考，联想要快；留空则不发这个参数
reasoning_effort = "none"
# 密钥：填在这里，或留空并设置 api_key_env 指定的环境变量（偏好设置里填的密钥写进配置同目录的 .env）
# api_key = ""
api_key_env = "QINGJIAN_API_KEY"
# 单次请求超时（毫秒）、停止敲键多久后才发请求（毫秒）
timeout_ms = 5000
debounce_ms = 300
# 光标前 / 后最多发多少个字符——这是发往云端的上下文上限
lookback = 64
lookahead = 32
# 云端词到了补进候选窗口第一页末尾几格（比如 2 就是 8、9 两格），前面的本地候选不动；0 表示不要云端词
slots = 2
# 组句中除了词候选还要不要整句补全（preedit 右侧，Tab 接受）
sentence = true
"#
);

impl Config {
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
        toml::from_str(&source).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })
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
            source,
        })?;
        // 分节不存在时先建成标准表，否则 toml_edit 会写成顶层的行内表 `predict = { enabled = true }`
        if !document.get(section).is_some_and(|item| item.is_table()) {
            document[section] = toml_edit::table();
        }
        document[section][key] = toml_edit::value(value);
        // 写临时文件再改名：输入法进程随时可能被杀，不能留半个配置文件
        qingjian_core::storage::write_atomic_str(path, &document.to_string()).map_err(|source| {
            ConfigError::Write {
                path: path.to_owned(),
                source,
            }
        })
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
            source,
        })?;
        if !document.get(section).is_some_and(|item| item.is_table()) {
            document[section] = toml_edit::table();
        }
        let mut array = toml_edit::Array::new();
        for value in values {
            array.push(value.as_ref());
        }
        document[section][key] = toml_edit::value(array);
        qingjian_core::storage::write_atomic_str(path, &document.to_string()).map_err(|source| {
            ConfigError::Write {
                path: path.to_owned(),
                source,
            }
        })
    }

    /// 文件不存在时写出模板，返回是否写了。
    pub fn write_template_if_missing(path: &Path) -> Result<bool, ConfigError> {
        if path.exists() {
            return Ok(false);
        }
        qingjian_core::storage::write_atomic_str(path, TEMPLATE).map_err(|source| {
            ConfigError::Write {
                path: path.to_owned(),
                source,
            }
        })?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_parses_to_defaults() {
        let config: Config = toml::from_str(TEMPLATE).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn partial_file_keeps_other_defaults() {
        let config: Config = toml::from_str("[predict]\nenabled = true\nlookback = 10\n").unwrap();
        assert!(config.predict.enabled);
        assert_eq!(config.predict.lookback, 10);
        assert_eq!(config.predict.model, "deepseek-v4-flash");
        assert_eq!(config.predict.reasoning_effort, "none");
        assert_eq!(config.predict.api_key_env, "QINGJIAN_API_KEY");
    }

    #[test]
    fn fuzzy_section_parses() {
        let config: Config = toml::from_str("[fuzzy]\nz_zh = true\nan_ang = true\n").unwrap();
        assert!(config.fuzzy.z_zh && config.fuzzy.an_ang && !config.fuzzy.n_l);
        assert!(config.fuzzy.any());
    }

    #[test]
    fn general_and_shortcut_sections_parse() {
        let config: Config = toml::from_str(
            "[general]\npage_size = 5\npage_keys = \"[]\"\ntheme = \"dark\"\nlayout = \"horizontal\"\npreedit = \"window\"\n[shortcut]\nexpression = \"i\"\n",
        )
        .unwrap();
        assert_eq!(config.general.page_size(), 5);
        assert_eq!(config.general.page_keys(), ('[', ']'));
        assert_eq!(config.general.theme, ThemeMode::Dark);
        assert_eq!(config.general.layout, LayoutMode::Horizontal);
        assert_eq!(config.general.preedit, PreeditMode::Window);
        assert_eq!(config.general.learning_language, "en");
        assert!(config.general.english_candidates);
        assert_eq!(config.general.shuangpin(), None);
        assert_eq!(config.general.log_level, LogLevel::Info);
        assert_eq!(config.shortcut.mode.expression, 'i');
        assert_eq!(config.shortcut.mode.question, 'u');
        assert_eq!(config.shortcut.translation, Modifiers::OPTION);
    }

    #[test]
    fn set_value_writes_strings_and_integers() {
        let path = std::env::temp_dir().join("qingjian-config-set-value-test.toml");
        let _ = std::fs::remove_file(&path);
        Config::set_value(&path, "general", "page_size", 5i64).unwrap();
        Config::set_value(&path, "general", "theme", "dark").unwrap();
        Config::set_value(&path, "shortcut", "question", "i").unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.general.page_size, 5);
        assert_eq!(config.general.theme, ThemeMode::Dark);
        assert_eq!(config.shortcut.mode.question, 'i');
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn set_bool_keeps_comments_and_flips_only_that_key() {
        let path = std::env::temp_dir().join("qingjian-config-set-bool-test.toml");
        std::fs::write(
            &path,
            "# 头注释\n[fuzzy]\n# 说明\nz_zh = false\nn_l = true\n",
        )
        .unwrap();
        Config::set_bool(&path, "fuzzy", "z_zh", true).unwrap();
        Config::set_bool(&path, "predict", "enabled", true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with("# 头注释\n[fuzzy]\n# 说明\nz_zh = true\nn_l = true\n"),
            "{text}"
        );
        let config = Config::load(&path).unwrap();
        assert!(config.fuzzy.z_zh && config.fuzzy.n_l && config.predict.enabled);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn set_bool_starts_from_template_when_missing() {
        let path = std::env::temp_dir().join("qingjian-config-set-bool-missing-test.toml");
        let _ = std::fs::remove_file(&path);
        Config::set_bool(&path, "predict", "enabled", true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# 青简输入法配置"));
        assert!(Config::load(&path).unwrap().predict.enabled);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn set_bool_refuses_broken_file() {
        let path = std::env::temp_dir().join("qingjian-config-set-bool-broken-test.toml");
        std::fs::write(&path, "[fuzzy\nz_zh = false\n").unwrap();
        assert!(matches!(
            Config::set_bool(&path, "fuzzy", "z_zh", true),
            Err(ConfigError::Edit { .. })
        ));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[fuzzy\nz_zh = false\n"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_default() {
        let path = std::env::temp_dir().join("qingjian-config-missing-test.toml");
        let _ = std::fs::remove_file(&path);
        assert_eq!(Config::load(&path).unwrap(), Config::default());
    }
}
