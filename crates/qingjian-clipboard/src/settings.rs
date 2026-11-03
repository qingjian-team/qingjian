//! 从输入法的配置文件里读剪贴板用的几个键：`[clipboard] enabled`、`edge`、`paused`、`excluded_apps`、`[shortcut] clipboard`。
//! 只读这几个键，不依赖输入法的配置库，剪贴板才能单独拆出去。写错的键按缺省算，不影响别的键。

use std::path::Path;

use serde::Deserialize;

use crate::edge::Edge;
use crate::error::ClipboardError;

/// 呼出面板的组合键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub control: bool,

    pub shift: bool,

    pub alt: bool,

    /// Windows 键。
    pub win: bool,

    /// 字母、数字或句号（小写）。
    pub key: char,
}

impl Hotkey {
    pub const DEFAULT: Self = Self {
        control: true,
        shift: false,
        alt: true,
        win: false,
        key: 'v',
    };

    /// 配置的写法：`ctrl+alt+v`，修饰键也认 `control`、`option`、`cmd`、`win`，顺序随意；至少一个修饰键。
    pub fn parse(text: &str) -> Option<Self> {
        let (modifiers, key) = text.trim().rsplit_once('+')?;
        let mut chars = key.trim().chars();
        let (Some(key), None) = (chars.next(), chars.next()) else {
            return None;
        };
        if !key.is_ascii_alphanumeric() && key != '.' {
            return None;
        }
        let mut hotkey = Self {
            control: false,
            shift: false,
            alt: false,
            win: false,
            key: key.to_ascii_lowercase(),
        };
        for token in modifiers
            .split(['+', ' '])
            .filter(|token| !token.is_empty())
        {
            match token.to_ascii_lowercase().as_str() {
                "option" | "alt" | "⌥" => hotkey.alt = true,
                "shift" | "⇧" => hotkey.shift = true,
                "control" | "ctrl" | "⌃" => hotkey.control = true,
                "command" | "cmd" | "super" | "win" | "⌘" => hotkey.win = true,
                _ => return None,
            }
        }
        (hotkey.control || hotkey.shift || hotkey.alt || hotkey.win).then_some(hotkey)
    }
}

/// 缺省最多留多少条，手改配置时不能超出的范围。
const DEFAULT_HISTORY_LIMIT: usize = 1000;

const HISTORY_LIMIT_RANGE: std::ops::RangeInclusive<i64> = 1..=100_000;

/// 缺省不记录的程序，与输入法配置的缺省一致（设置程序的测试核对）。
const DEFAULT_EXCLUDED: [&str; 4] = ["KeePass", "KeePassXC", "1Password", "Bitwarden"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipSettings {
    pub enabled: bool,

    pub edge: Edge,

    pub hotkey: Hotkey,

    /// 最多留多少条。
    pub history_limit: usize,

    /// 暂时不记录。
    pub paused: bool,

    /// 不记录的程序名。
    pub excluded_apps: Vec<String>,

    /// 同步到局域网里配对的设备。缺省关，关着时不监听端口、不公布自己。
    pub sync_enabled: bool,

    /// 同步时屏幕边缘亮霓虹灯。
    pub sync_neon: bool,

    /// 给别的设备看的名字；空着用默认名。
    pub device_name: String,

    /// 配置里写的界面语言（`[general] ui_language`），见 [`Self::locale`]。
    pub language: String,
}

/// 路径里的程序名，去掉 `.exe`，都小写。路径分隔符两种都认：这个 crate 平台无关，来源路径是系统给的。
fn app_name(path: &str) -> String {
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_lowercase();
    match name.strip_suffix(".exe") {
        Some(stem) => stem.to_owned(),
        None => name,
    }
}

impl Default for ClipSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            edge: Edge::Bottom,
            hotkey: Hotkey::DEFAULT,
            language: String::new(),
            history_limit: DEFAULT_HISTORY_LIMIT,
            paused: false,
            sync_enabled: false,
            sync_neon: true,
            device_name: String::new(),
            excluded_apps: DEFAULT_EXCLUDED
                .iter()
                .map(|app| (*app).to_owned())
                .collect(),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct File {
    general: GeneralSection,

    clipboard: ClipboardSection,

    shortcut: ShortcutSection,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct GeneralSection {
    ui_language: Option<toml::Value>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct ClipboardSection {
    enabled: Option<toml::Value>,

    edge: Option<toml::Value>,

    history_limit: Option<toml::Value>,

    paused: Option<toml::Value>,

    excluded_apps: Option<toml::Value>,

    sync_enabled: Option<toml::Value>,

    sync_neon: Option<toml::Value>,

    device_name: Option<toml::Value>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct ShortcutSection {
    clipboard: Option<toml::Value>,
}

impl ClipSettings {
    /// 交给界面语言的标签：没写是简体中文，`system` 跟随系统（空串），与输入法的规则相同。
    pub fn locale(&self) -> &str {
        match self.language.trim() {
            "" => "zh-Hans",
            "system" => "",
            tag => tag,
        }
    }

    /// 改配置文件里的一个 `[clipboard]` 开关（托盘菜单用），别的内容与注释原样留着。
    pub fn write_flag(path: &Path, key: &str, value: bool) -> Result<(), ClipboardError> {
        Self::write_value(path, key, toml_edit::value(value))
    }

    /// 改设备名，写法同 [`Self::write_flag`]。
    pub fn write_device_name(path: &Path, name: &str) -> Result<(), ClipboardError> {
        Self::write_value(path, "device_name", toml_edit::value(name))
    }

    fn write_value(path: &Path, key: &str, value: toml_edit::Item) -> Result<(), ClipboardError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        let mut document: toml_edit::DocumentMut = text
            .parse()
            .map_err(|_| ClipboardError::BadConfig(path.display().to_string()))?;
        document["clipboard"].or_insert(toml_edit::table())[key] = value;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, document.to_string())?;
        Ok(())
    }

    /// 从 `source`（程序路径）复制的内容要不要记。
    pub fn records(&self, source: &str) -> bool {
        if self.paused {
            return false;
        }
        let name = app_name(source);
        !self.excluded_apps.iter().any(|app| app_name(app) == name)
    }

    /// 解析配置文件的内容；不是合法的 TOML 时为 `None`（文件写到一半这类，调用方保持原样）。
    pub fn parse(text: &str) -> Option<Self> {
        let file: File = toml::from_str(text).ok()?;
        let defaults = Self::default();
        let text_of = |value: &Option<toml::Value>| {
            value
                .as_ref()
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
        };
        Some(Self {
            enabled: file
                .clipboard
                .enabled
                .as_ref()
                .and_then(toml::Value::as_bool)
                .unwrap_or(defaults.enabled),
            edge: text_of(&file.clipboard.edge)
                .and_then(|edge| Edge::parse(&edge))
                .unwrap_or(defaults.edge),
            hotkey: text_of(&file.shortcut.clipboard)
                .and_then(|hotkey| Hotkey::parse(&hotkey))
                .unwrap_or(defaults.hotkey),
            language: text_of(&file.general.ui_language).unwrap_or(defaults.language),
            history_limit: file
                .clipboard
                .history_limit
                .as_ref()
                .and_then(toml::Value::as_integer)
                .filter(|limit| HISTORY_LIMIT_RANGE.contains(limit))
                .map_or(defaults.history_limit, |limit| limit as usize),
            paused: file
                .clipboard
                .paused
                .as_ref()
                .and_then(toml::Value::as_bool)
                .unwrap_or(defaults.paused),
            sync_enabled: file
                .clipboard
                .sync_enabled
                .as_ref()
                .and_then(toml::Value::as_bool)
                .unwrap_or(defaults.sync_enabled),
            sync_neon: file
                .clipboard
                .sync_neon
                .as_ref()
                .and_then(toml::Value::as_bool)
                .unwrap_or(defaults.sync_neon),
            device_name: text_of(&file.clipboard.device_name).unwrap_or(defaults.device_name),
            excluded_apps: file
                .clipboard
                .excluded_apps
                .as_ref()
                .and_then(toml::Value::as_array)
                .map_or(defaults.excluded_apps, |apps| {
                    apps.iter()
                        .filter_map(toml::Value::as_str)
                        .map(str::to_owned)
                        .collect()
                }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ClipSettings, Edge, Hotkey};

    #[test]
    fn reads_what_the_settings_app_writes() {
        let settings = ClipSettings::parse(
            "[clipboard]\nenabled = true\nedge = \"left\"\n[shortcut]\nclipboard = \"control+shift+option+k\"\n",
        )
        .unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.edge, Edge::Left);
        assert_eq!(
            settings.hotkey,
            Hotkey {
                control: true,
                shift: true,
                alt: true,
                win: false,
                key: 'k',
            }
        );
    }

    #[test]
    fn missing_or_wrong_keys_fall_back_one_by_one() {
        assert_eq!(ClipSettings::parse(""), Some(ClipSettings::default()));
        let settings = ClipSettings::parse(
            "[clipboard]\nenabled = \"yes\"\nedge = \"middle\"\n[shortcut]\nclipboard = \"v\"\n",
        )
        .unwrap();
        assert_eq!(settings, ClipSettings::default());
        assert_eq!(ClipSettings::parse("[clipboard"), None);
    }

    #[test]
    fn other_sections_of_the_config_do_not_matter() {
        let settings = ClipSettings::parse(
            "[general]\npage_size = 5\n[shortcut]\npredict = \"ctrl+alt+j\"\nclipboard = \"win+shift+.\"\n",
        )
        .unwrap();
        assert_eq!(settings.hotkey.key, '.');
        assert!(settings.hotkey.win && settings.hotkey.shift);
    }

    #[test]
    fn paused_and_excluded_apps_are_not_recorded() {
        let settings =
            ClipSettings::parse("[clipboard]\nexcluded_apps = [\"KeePass\", \"secret.exe\", 3]\n")
                .unwrap();
        assert_eq!(settings.excluded_apps, ["KeePass", "secret.exe"]);
        assert!(!settings.records(r"C:\Program Files\KeePass\keepass.EXE"));
        assert!(!settings.records("/usr/bin/secret"));
        assert!(settings.records(r"C:\Windows\notepad.exe"));
        assert!(settings.records(""));
        let paused = ClipSettings {
            paused: true,
            ..settings
        };
        assert!(!paused.records(r"C:\Windows\notepad.exe"));
        // 没写这个键用缺省的密码管理器，写了空表就是都记
        assert!(!ClipSettings::default().records("Bitwarden.exe"));
        let none = ClipSettings::parse("[clipboard]\nexcluded_apps = []\n").unwrap();
        assert!(none.records("Bitwarden.exe"));
    }

    #[test]
    fn writing_a_flag_keeps_everything_else() {
        let path = std::env::temp_dir().join(format!("qj-clip-flag-{}.toml", std::process::id()));
        std::fs::write(&path, "# mine\n[general]\npage_size = 5\n").unwrap();
        ClipSettings::write_flag(&path, "paused", true).unwrap();
        ClipSettings::write_flag(&path, "enabled", false).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# mine") && text.contains("page_size = 5"));
        assert!(ClipSettings::parse(&text).unwrap().paused);
        // 配置写坏了不覆盖
        std::fs::write(&path, "[general").unwrap();
        assert!(ClipSettings::write_flag(&path, "paused", false).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[general");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn history_limit_falls_back_when_out_of_range() {
        let limit = |text: &str| ClipSettings::parse(text).unwrap().history_limit;
        assert_eq!(limit("[clipboard]\nhistory_limit = 500\n"), 500);
        assert_eq!(limit("[clipboard]\nhistory_limit = 0\n"), 1000);
        assert_eq!(limit("[clipboard]\nhistory_limit = \"many\"\n"), 1000);
        assert_eq!(limit(""), 1000);
    }

    #[test]
    fn locale_follows_the_same_rule_as_the_ime() {
        let with = |language: &str| ClipSettings {
            language: language.to_owned(),
            ..ClipSettings::default()
        };
        assert_eq!(with("").locale(), "zh-Hans");
        assert_eq!(with("system").locale(), "");
        assert_eq!(with("en").locale(), "en");
    }

    #[test]
    fn hotkey_needs_a_modifier_and_one_key() {
        assert_eq!(Hotkey::parse("ctrl+alt+v"), Some(Hotkey::DEFAULT));
        for bad in ["v", "ctrl+", "ctrl+vv", "ctrl+-", "hyper+v", "+v"] {
            assert_eq!(Hotkey::parse(bad), None, "{bad}");
        }
    }
}
