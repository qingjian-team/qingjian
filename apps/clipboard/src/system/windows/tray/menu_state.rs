//! [`MenuState`]：托盘菜单上显示什么。

pub struct MenuState {
    pub paused: bool,

    /// 同目录有设置程序。
    pub has_settings: bool,

    /// 单独运行：开机启动归自己管（由输入法拉起时归输入法）。
    pub standalone: bool,
}
