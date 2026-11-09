//! [`Choice`]：托盘右键菜单选了什么。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Open,
    TogglePause,
    OpenSettings,
    ToggleAutostart,
    Quit,
}
