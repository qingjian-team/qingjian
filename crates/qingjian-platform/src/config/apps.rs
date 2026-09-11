use serde::{Deserialize, Serialize};

/// 缺省不给英文候选的应用（macOS，按 bundle identifier）：终端、代码编辑器、IDE。这些地方的英文候选窗口会挡住应用自己的补全，
/// vim / nano 里 Tab 与方向键又都有别的意思。`*` 结尾是前缀匹配。
pub const DEFAULT_ENGLISH_CANDIDATES_OFF_MACOS: &[&str] = &[
    "com.apple.Terminal",
    "com.googlecode.iterm2",
    "dev.warp.Warp-Stable",
    "com.mitchellh.ghostty",
    "io.alacritty",
    "net.kovidgoyal.kitty",
    "com.microsoft.VSCode",
    "com.todesktop.230313mzl4w4u92", // Cursor
    "dev.zed.Zed",
    "com.jetbrains.*",
    "org.vim.MacVim",
    "com.sublimetext.*",
    "com.apple.dt.Xcode",
    "com.neovide.neovide",
];

/// 缺省不给英文候选的应用（Windows，按宿主进程的 exe 文件名）。输入法 DLL 加载在拥有窗口的那个进程里：
/// 经典控制台的窗口属于 `conhost.exe`（cmd / PowerShell 自己没有窗口），Windows Terminal 是 `WindowsTerminal.exe`。
/// JetBrains 各 IDE 的 exe 名没有共同前缀，只能逐个列。
pub const DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS: &[&str] = &[
    "conhost.exe",
    "WindowsTerminal.exe",
    "alacritty.exe",
    "wezterm-gui.exe",
    "mintty.exe", // Git Bash
    "Code.exe",
    "Code - Insiders.exe",
    "Cursor.exe",
    "zed.exe",
    "idea64.exe",
    "pycharm64.exe",
    "clion64.exe",
    "rustrover64.exe",
    "goland64.exe",
    "rider64.exe",
    "webstorm64.exe",
    "phpstorm64.exe",
    "datagrip64.exe",
    "devenv.exe", // Visual Studio
    "sublime_text.exe",
    "notepad++.exe",
    "gvim.exe",
    "neovide.exe",
];

/// 本平台的缺省名单：macOS 上是 bundle identifier，Windows 上是 exe 文件名。
#[cfg(windows)]
pub const DEFAULT_ENGLISH_CANDIDATES_OFF: &[&str] = DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS;

/// 本平台的缺省名单：macOS 上是 bundle identifier，Windows 上是 exe 文件名。
#[cfg(not(windows))]
pub const DEFAULT_ENGLISH_CANDIDATES_OFF: &[&str] = DEFAULT_ENGLISH_CANDIDATES_OFF_MACOS;

/// 配置文件 `[apps]` 分节：按应用改行为。应用的标识 macOS 上是 bundle identifier，Windows 上是宿主进程的 exe 文件名。
///
/// 现在只有一项：哪些应用里英文模式不给候选（纯直通）。以后按应用定 preedit 模式等也放这里。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppsConfig {
    /// 英文模式（Caps Lock）下不给候选的应用。条目是 bundle identifier（`com.jetbrains.*`）或 exe 文件名（`Code.exe`），
    /// `*` 结尾按前缀匹配。全局开关 `[general] english_candidates` 关着时这里不起作用。
    pub english_candidates_off: Vec<String>,
}

impl Default for AppsConfig {
    fn default() -> Self {
        Self::with_english_candidates_off(DEFAULT_ENGLISH_CANDIDATES_OFF)
    }
}

impl AppsConfig {
    /// 用给定名单构造（缺省名单分平台，测试里要指定哪一份）。
    pub fn with_english_candidates_off(apps: &[&str]) -> Self {
        Self {
            english_candidates_off: apps.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    /// 这个应用里英文模式要不要关掉候选。`app` 不认识（应用没给）按不关。
    pub fn english_candidates_off(&self, app: &str) -> bool {
        self.english_candidates_off
            .iter()
            .any(|pattern| matches_app(pattern, app))
    }

    /// 列表里有没有东西（偏好设置的勾选框据此显示）。
    pub fn has_english_candidates_off(&self) -> bool {
        !self.english_candidates_off.is_empty()
    }
}

/// `pattern` 是完整的应用标识，或 `*` 结尾的前缀。不区分大小写（bundle identifier 与 Windows 文件名本身都不区分）。
fn matches_app(pattern: &str, app: &str) -> bool {
    let pattern = pattern.trim();
    match pattern.strip_suffix('*') {
        Some(prefix) => {
            app.len() >= prefix.len() && app[..prefix.len()].eq_ignore_ascii_case(prefix)
        }
        None => pattern.eq_ignore_ascii_case(app),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_list_covers_terminals_and_ides_with_prefix_patterns() {
        let apps = AppsConfig::with_english_candidates_off(DEFAULT_ENGLISH_CANDIDATES_OFF_MACOS);
        assert!(apps.english_candidates_off("com.apple.Terminal"));
        assert!(apps.english_candidates_off("com.jetbrains.intellij"));
        assert!(apps.english_candidates_off("com.jetbrains.rustrover"));
        assert!(apps.english_candidates_off("COM.MICROSOFT.VSCODE"));
        assert!(!apps.english_candidates_off("com.apple.TextEdit"));
        assert!(!apps.english_candidates_off("com.jetbrains"));
        assert!(!apps.english_candidates_off(""));
    }

    #[test]
    fn windows_list_matches_exe_names_case_insensitively() {
        let apps = AppsConfig::with_english_candidates_off(DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS);
        assert!(apps.english_candidates_off("conhost.exe"));
        assert!(apps.english_candidates_off("WindowsTerminal.exe"));
        assert!(apps.english_candidates_off("code.exe"));
        assert!(apps.english_candidates_off("RustRover64.exe"));
        assert!(!apps.english_candidates_off("notepad.exe"));
        assert!(!apps.english_candidates_off("Code"));
    }

    #[test]
    fn default_list_follows_the_platform() {
        let apps = AppsConfig::default();
        assert_eq!(
            apps.english_candidates_off("Code.exe"),
            cfg!(windows),
            "Windows 缺省名单按 exe 名"
        );
        assert_eq!(
            apps.english_candidates_off("com.microsoft.VSCode"),
            !cfg!(windows),
            "macOS 缺省名单按 bundle identifier"
        );
    }

    #[test]
    fn empty_list_turns_the_feature_off() {
        let apps: AppsConfig = toml::from_str("english_candidates_off = []").unwrap();
        assert!(!apps.has_english_candidates_off());
        assert!(!apps.english_candidates_off("com.apple.Terminal"));
    }

    #[test]
    fn prefix_pattern_needs_the_whole_prefix() {
        assert!(matches_app("com.jetbrains.*", "com.jetbrains.goland"));
        assert!(!matches_app("com.jetbrains.*", "com.jetbrain"));
        assert!(matches_app("*", "anything"));
    }
}
