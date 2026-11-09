//! 主题列表里的一项，设置页按它列出主题卡片。

use qingjian_render::Theme;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ThemeEntry {
    /// 存进设置的 id（同 PC 配置的 `[general] theme`）。
    pub id: String,

    /// 显示名。
    pub name: String,
}

impl ThemeEntry {
    pub fn new(theme: &Theme) -> Self {
        Self {
            id: theme.id().to_owned(),
            name: theme.name().to_owned(),
        }
    }
}
