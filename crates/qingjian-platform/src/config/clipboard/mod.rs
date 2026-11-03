//! `[clipboard]` 分节：青简剪贴板（实验功能）。快捷键按约定在 `[shortcut] clipboard`。
//! 剪贴板进程不依赖这个 crate，自己读这几个键（`qingjian_clipboard::ClipSettings`），改写法要两边一起改。

mod edge;

use serde::{Deserialize, Serialize};

pub use edge::ClipboardEdge;

/// 设置界面列出的最大条数，缺省 1000。
pub const CLIPBOARD_HISTORY_LIMITS: [u32; 4] = [100, 500, 1000, 5000];

/// 缺省不记录的程序：密码管理器。它们多数已给剪贴板内容打了「别记录」的标记，这里再兜一层。
pub const DEFAULT_CLIPBOARD_EXCLUDED_APPS: [&str; 4] =
    ["KeePass", "KeePassXC", "1Password", "Bitwarden"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipboardConfig {
    /// 开着时输入法 Server 拉起剪贴板进程，关了它自己退出。缺省关。
    pub enabled: bool,

    pub edge: ClipboardEdge,

    /// 最多留多少条，超了删最旧的。
    pub history_limit: u32,

    /// 暂时不记录新复制的内容；已有的照常能用。
    pub paused: bool,

    /// 从这些程序复制的内容不记录，写程序名，不分大小写，带不带 `.exe` 都行。
    pub excluded_apps: Vec<String>,

    /// 同步到局域网里配对的设备。缺省关，关着时不监听端口、不公布自己。
    pub sync_enabled: bool,

    /// 同步时屏幕边缘亮霓虹灯。
    pub sync_neon: bool,

    /// 给别的设备看的名字；空着用默认名。
    pub device_name: String,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            edge: ClipboardEdge::default(),
            history_limit: CLIPBOARD_HISTORY_LIMITS[2],
            paused: false,
            sync_enabled: false,
            sync_neon: true,
            device_name: String::new(),
            excluded_apps: DEFAULT_CLIPBOARD_EXCLUDED_APPS
                .iter()
                .map(|app| (*app).to_owned())
                .collect(),
        }
    }
}
