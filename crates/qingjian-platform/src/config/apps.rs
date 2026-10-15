use serde::{Deserialize, Serialize};

/// 缺省给英文候选的应用（macOS，按 bundle identifier）：浏览器、聊天、办公与笔记这类写成段文字的地方。
/// 白名单而不是黑名单：设计、3D、CAD、视频剪辑、游戏都靠单键快捷键（Photoshop 的 W / H / V），
/// 英文候选会把字母收进组句，这类应用列不完。`*` 结尾是前缀匹配。
pub const DEFAULT_ENGLISH_CANDIDATES_ON_MACOS: &[&str] = &[
    "com.apple.Safari",
    "com.google.Chrome*",
    "org.mozilla.firefox",
    "com.microsoft.edgemac*",
    "company.thebrowser.Browser", // Arc
    "com.brave.Browser*",
    "com.apple.TextEdit",
    "com.apple.Notes",
    "com.apple.mail",
    "com.apple.MobileSMS",
    "com.apple.iWork.Pages",
    "com.apple.iWork.Keynote",
    "com.microsoft.Word",
    "com.microsoft.Powerpoint",
    "com.microsoft.Outlook",
    "com.microsoft.onenote.mac",
    "com.microsoft.teams2",
    "com.kingsoft.wpsoffice.mac",
    "com.tencent.xinWeChat",
    "com.tencent.qq",
    "com.tencent.WeWorkMac",
    "com.alibaba.DingTalkMac",
    "com.electron.lark", // 飞书
    "ru.keepcoder.Telegram",
    "com.tdesktop.Telegram",
    "com.hnc.Discord",
    "com.tinyspeck.slackmacgap",
    "md.obsidian",
    "notion.id",
    "abnerworks.Typora",
];

/// 缺省给英文候选的应用（Windows，按宿主进程的 exe 文件名）。理由同 [`DEFAULT_ENGLISH_CANDIDATES_ON_MACOS`]。
pub const DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS: &[&str] = &[
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "brave.exe",
    "opera.exe",
    "vivaldi.exe",
    "notepad.exe",
    "WINWORD.EXE",
    "POWERPNT.EXE",
    "OUTLOOK.EXE",
    "ONENOTE.EXE",
    "ms-teams.exe",
    "wps.exe",
    "wpp.exe",
    "Weixin.exe", // 微信 4.x
    "WeChat.exe",
    "QQ.exe",
    "WXWork.exe", // 企业微信
    "DingTalk.exe",
    "Feishu.exe",
    "Lark.exe",
    "Telegram.exe",
    "Discord.exe",
    "slack.exe",
    "thunderbird.exe",
    "Obsidian.exe",
    "Notion.exe",
    "Typora.exe",
];

/// 缺省给英文候选的应用（Linux，按 fcitx5 的 program 名：X11 是 WM_CLASS，Wayland 是 app_id）。
/// 理由同 [`DEFAULT_ENGLISH_CANDIDATES_ON_MACOS`]；匹配大小写不敏感，所以大小写变体不用重复列。
pub const DEFAULT_ENGLISH_CANDIDATES_ON_LINUX: &[&str] = &[
    "firefox",
    "org.mozilla.firefox",
    "google-chrome",
    "chromium",
    "chromium-browser",
    "microsoft-edge",
    "brave-browser",
    "thunderbird",
    "org.mozilla.thunderbird",
    "libreoffice*",
    "soffice",
    "wps",
    "wpp",
    "gedit",
    "org.gnome.gedit",
    "org.gnome.TextEditor",
    "kate",
    "org.kde.kate",
    "kwrite",
    "mousepad",
    "wechat",
    "telegram-desktop",
    "org.telegram.desktop",
    "discord",
    "slack",
    "obsidian",
    "typora",
];

/// 本平台的缺省白名单：macOS 上是 bundle identifier，Windows 上是 exe 文件名，Linux 上是 fcitx5 的 program 名。
#[cfg(windows)]
pub const DEFAULT_ENGLISH_CANDIDATES_ON: &[&str] = DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS;

/// 本平台的缺省白名单：macOS 上是 bundle identifier，Windows 上是 exe 文件名，Linux 上是 fcitx5 的 program 名。
#[cfg(target_os = "macos")]
pub const DEFAULT_ENGLISH_CANDIDATES_ON: &[&str] = DEFAULT_ENGLISH_CANDIDATES_ON_MACOS;

/// 本平台的缺省白名单：macOS 上是 bundle identifier，Windows 上是 exe 文件名，Linux 上是 fcitx5 的 program 名。
#[cfg(not(any(windows, target_os = "macos")))]
pub const DEFAULT_ENGLISH_CANDIDATES_ON: &[&str] = DEFAULT_ENGLISH_CANDIDATES_ON_LINUX;

/// 配置文件 `[apps]` 分节：按应用改行为。应用的标识 macOS 上是 bundle identifier，Windows 上是宿主进程的 exe 文件名，
/// Linux 上是 fcitx5 认到的应用名。
///
/// 现在只管英文模式给不给候选：先看白名单 `english_candidates_on`，再用 `english_candidates_off` 排除。
/// 全局开关 `[general] english_candidates` 关着时两份名单都不起作用。以后按应用定 preedit 模式等也放这里。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppsConfig {
    /// 英文模式下给候选的应用，`*` 结尾按前缀匹配，单写 `"*"` 就是处处都给。
    pub english_candidates_on: Vec<String>,

    /// 从白名单里排除的应用（配合 `"*"` 用：处处都给、只有这几个不给）。缺省空。
    pub english_candidates_off: Vec<String>,
}

impl Default for AppsConfig {
    fn default() -> Self {
        Self::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON)
    }
}

impl AppsConfig {
    /// 用给定白名单构造（缺省名单分平台，测试里要指定哪一份）。
    pub fn with_english_candidates_on(apps: &[&str]) -> Self {
        Self {
            english_candidates_on: apps.iter().map(|s| (*s).to_owned()).collect(),
            english_candidates_off: Vec::new(),
        }
    }

    /// 这个应用里英文模式给不给候选。`app` 不认识（应用没报）时只有 `"*"` 给：认不出来的地方宁可原样交给应用。
    pub fn english_candidates_on(&self, app: Option<&str>) -> bool {
        let Some(app) = app else {
            return self.english_candidates_everywhere();
        };
        self.english_candidates_on
            .iter()
            .any(|pattern| matches_app(pattern, app))
            && !self
                .english_candidates_off
                .iter()
                .any(|pattern| matches_app(pattern, app))
    }

    /// 白名单是不是「处处都给」（`"*"`）：偏好设置里「只在常用写字应用里给候选」的勾选框据此显示。
    pub fn english_candidates_everywhere(&self) -> bool {
        self.english_candidates_on
            .iter()
            .any(|pattern| pattern.trim() == "*")
    }
}

/// `pattern` 是完整的应用标识，或 `*` 结尾的前缀。不区分大小写（bundle identifier 与 Windows 文件名本身都不区分）。
fn matches_app(pattern: &str, app: &str) -> bool {
    let pattern = pattern.trim();
    match pattern.strip_suffix('*') {
        Some(prefix) => {
            // 应用名不保证是 ASCII（Windows 的 exe 文件名、Linux 的 app_id / WM_CLASS 都可以带非 ASCII 字符）：
            // 按字节切片会切在字符中间 panic，`get` 切不到就是不匹配。
            app.get(..prefix.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
        }
        None => pattern.eq_ignore_ascii_case(app),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_list_covers_writing_apps_with_prefix_patterns() {
        let apps = AppsConfig::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON_MACOS);
        assert!(apps.english_candidates_on(Some("com.apple.TextEdit")));
        assert!(apps.english_candidates_on(Some("com.google.Chrome.canary")));
        assert!(apps.english_candidates_on(Some("COM.TENCENT.XINWECHAT")));
        assert!(!apps.english_candidates_on(Some("com.adobe.Photoshop")));
        assert!(!apps.english_candidates_on(Some("com.apple.Terminal")));
        assert!(!apps.english_candidates_on(Some("")));
        assert!(!apps.english_candidates_on(None));
    }

    #[test]
    fn windows_list_matches_exe_names() {
        let apps = AppsConfig::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS);
        assert!(apps.english_candidates_on(Some("Weixin.exe")));
        assert!(apps.english_candidates_on(Some("winword.exe")));
        assert!(!apps.english_candidates_on(Some("Photoshop.exe")));
        assert!(!apps.english_candidates_on(Some("SLDWORKS.exe")));
        assert!(!apps.english_candidates_on(Some("Code.exe")));
        assert!(!apps.english_candidates_on(Some("chrome")));
    }

    #[test]
    fn linux_defaults_match_program_names() {
        let apps = AppsConfig::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON_LINUX);
        assert!(apps.english_candidates_on(Some("firefox")));
        assert!(
            apps.english_candidates_on(Some("ORG.KDE.Kate")),
            "匹配大小写不敏感"
        );
        assert!(
            apps.english_candidates_on(Some("libreoffice-writer")),
            "* 前缀匹配"
        );
        assert!(!apps.english_candidates_on(Some("konsole")));
        assert!(!apps.english_candidates_on(Some("blender")));
    }

    #[test]
    fn default_list_follows_the_platform() {
        let apps = AppsConfig::default();
        assert_eq!(
            apps.english_candidates_on(Some("chrome.exe")),
            cfg!(windows),
            "Windows 缺省名单按 exe 名"
        );
        assert_eq!(
            apps.english_candidates_on(Some("com.apple.Safari")),
            cfg!(target_os = "macos"),
            "macOS 缺省名单按 bundle identifier"
        );
        assert_eq!(
            apps.english_candidates_on(Some("kate")),
            cfg!(not(any(windows, target_os = "macos"))),
            "Linux 缺省名单按 fcitx5 的 program 名"
        );
        assert!(!apps.english_candidates_everywhere());
    }

    #[test]
    fn star_means_everywhere_minus_the_off_list() {
        let apps: AppsConfig = toml::from_str(
            r#"english_candidates_on = ["*"]
english_candidates_off = ["Photoshop.exe"]"#,
        )
        .unwrap();
        assert!(apps.english_candidates_everywhere());
        assert!(apps.english_candidates_on(Some("anything.exe")));
        assert!(!apps.english_candidates_on(Some("photoshop.exe")));
        assert!(apps.english_candidates_on(None), "认不出应用也给");
    }

    #[test]
    fn empty_list_gives_candidates_nowhere() {
        let apps: AppsConfig = toml::from_str("english_candidates_on = []").unwrap();
        assert!(!apps.english_candidates_on(Some("com.apple.TextEdit")));
    }

    #[test]
    fn prefix_pattern_needs_the_whole_prefix() {
        assert!(matches_app("com.jetbrains.*", "com.jetbrains.goland"));
        assert!(!matches_app("com.jetbrains.*", "com.jetbrain"));
        assert!(matches_app("*", "anything"));
    }

    #[test]
    fn prefix_patterns_survive_multibyte_app_names() {
        // 名单里有 `*` 前缀项时，多字节应用名的第 prefix.len() 字节可能落在字符中间：
        // 必须判为不匹配，不许 panic。
        let apps = AppsConfig::with_english_candidates_on(&["jetbrains-*", "code"]);
        assert!(!apps.english_candidates_on(Some("日本語入力テスト")));
        assert!(!apps.english_candidates_on(Some("abc日本語")));
        assert!(!apps.english_candidates_on(Some("日本語")));
        assert!(
            apps.english_candidates_on(Some("jetbrains-idea")),
            "既有前缀匹配不受影响"
        );
    }
}
