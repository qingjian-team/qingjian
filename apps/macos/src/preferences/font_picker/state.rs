//! 字体选择器的状态：全部字族、过滤后可见的那些、控件引用，供回调独立读取。

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2_app_kit::{NSPopUpButton, NSPopover, NSSearchField, NSTableView};

use crate::preferences::setting::Setting;

pub(super) struct PickerState {
    /// 此实例修改的配置项，两个字族选择器互不影响。
    pub setting: Setting,

    /// 系统全部字族名，按系统给的顺序。
    pub families: Vec<String>,

    /// 过滤后显示的行：第 0 行永远是「系统默认」。
    pub visible: RefCell<Vec<String>>,

    /// 配置里当前的字族名（空为系统字体）。
    pub current: RefCell<String>,

    /// 页面上的按钮，显示当前字体名。
    pub button: RefCell<Option<Retained<NSPopUpButton>>>,

    /// 下拉框里的搜索框。
    pub search: RefCell<Option<Retained<NSSearchField>>>,

    /// 列表。
    pub table: RefCell<Option<Retained<NSTableView>>>,

    /// 装着列表的下拉框。
    pub popover: RefCell<Option<Retained<NSPopover>>>,

    /// 程序在同步选中行时为 true，此时选中变化不当作用户操作。
    pub syncing: Cell<bool>,
}
