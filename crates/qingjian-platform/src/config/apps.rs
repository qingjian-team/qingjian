use serde::{Deserialize, Serialize};

/// 缺省不给英文候选的应用：终端、代码编辑器、IDE。这些地方的英文候选窗口会挡住应用自己的补全，
/// vim / nano 里 Tab 与方向键又都有别的意思。按 bundle identifier 认，`*` 结尾是前缀匹配。
pub const DEFAULT_ENGLISH_CANDIDATES_OFF: &[&str] = &[
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

/// 配置文件 `[apps]` 分节：按应用（bundle identifier）改行为。
///
/// 现在只有一项：哪些应用里英文模式不给候选（纯直通）。以后按应用定 preedit 模式等也放这里。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppsConfig {
    /// 英文模式（Caps Lock）下不给候选的应用。条目是 bundle identifier，`*` 结尾按前缀匹配（`com.jetbrains.*`）。
    /// 全局开关 `[general] english_candidates` 关着时这里不起作用。
    pub english_candidates_off: Vec<String>,
}

impl Default for AppsConfig {
    fn default() -> Self {
        Self {
            english_candidates_off: DEFAULT_ENGLISH_CANDIDATES_OFF
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }
}

impl AppsConfig {
    /// 这个应用里英文模式要不要关掉候选。`bundle` 不认识（应用没给）按不关。
    pub fn english_candidates_off(&self, bundle: &str) -> bool {
        self.english_candidates_off
            .iter()
            .any(|pattern| matches_bundle(pattern, bundle))
    }

    /// 列表里有没有东西（偏好设置的勾选框据此显示）。
    pub fn has_english_candidates_off(&self) -> bool {
        !self.english_candidates_off.is_empty()
    }
}

/// `pattern` 是完整的 bundle identifier，或 `*` 结尾的前缀。不区分大小写（bundle identifier 本身就不区分）。
fn matches_bundle(pattern: &str, bundle: &str) -> bool {
    let pattern = pattern.trim();
    match pattern.strip_suffix('*') {
        Some(prefix) => {
            bundle.len() >= prefix.len() && bundle[..prefix.len()].eq_ignore_ascii_case(prefix)
        }
        None => pattern.eq_ignore_ascii_case(bundle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_list_covers_terminals_and_ides_with_prefix_patterns() {
        let apps = AppsConfig::default();
        assert!(apps.english_candidates_off("com.apple.Terminal"));
        assert!(apps.english_candidates_off("com.jetbrains.intellij"));
        assert!(apps.english_candidates_off("com.jetbrains.rustrover"));
        assert!(apps.english_candidates_off("COM.MICROSOFT.VSCODE"));
        assert!(!apps.english_candidates_off("com.apple.TextEdit"));
        assert!(!apps.english_candidates_off("com.jetbrains"));
        assert!(!apps.english_candidates_off(""));
    }

    #[test]
    fn empty_list_turns_the_feature_off() {
        let apps: AppsConfig = toml::from_str("english_candidates_off = []").unwrap();
        assert!(!apps.has_english_candidates_off());
        assert!(!apps.english_candidates_off("com.apple.Terminal"));
    }

    #[test]
    fn prefix_pattern_needs_the_whole_prefix() {
        assert!(matches_bundle("com.jetbrains.*", "com.jetbrains.goland"));
        assert!(!matches_bundle("com.jetbrains.*", "com.jetbrain"));
        assert!(matches_bundle("*", "anything"));
    }
}
