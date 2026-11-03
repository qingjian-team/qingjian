//! 右键菜单：几个菜单项的序号与 [`OpenedMenu`]。

/// 右键菜单的几项，序号与 `view::menu_items` 对应。
pub const MENU_PASTE: usize = 0;

pub const MENU_COPY: usize = 1;

pub const MENU_EDIT: usize = 2;

pub const MENU_DELETE: usize = 3;

/// 开着的右键菜单。
pub struct OpenedMenu {
    pub index: usize,

    pub position: (f32, f32),
}
