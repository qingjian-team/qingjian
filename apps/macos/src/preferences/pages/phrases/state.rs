use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSTableView};
use qingjian_core::CustomPhrase;
use std::cell::RefCell;

pub(super) struct TableState {
    /// 当前配置的只读显示副本，回调不借用 Host。
    pub rows: RefCell<Vec<CustomPhrase>>,

    /// 所属表格。
    pub table: RefCell<Option<Retained<NSTableView>>>,

    /// 无选中行时禁用编辑。
    pub edit: Retained<NSButton>,

    /// 无选中行时禁用删除。
    pub delete: Retained<NSButton>,
}
