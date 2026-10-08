//! 字体列表的数据源与代理：管下拉框的开合、按搜索框过滤、每行用它自己的字体画名字、用户选中一行就写配置并收起。

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSControlTextEditingDelegate, NSFont, NSFontManager, NSFontTraitMask, NSLineBreakMode,
    NSPopUpButton, NSPopover, NSSearchField, NSTableColumn, NSTableView, NSTableViewDataSource,
    NSTableViewDelegate, NSTextField, NSView,
};
use objc2_foundation::{
    NSIndexSet, NSInteger, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSRectEdge,
    NSSize, NSString,
};

use super::state::PickerState;
use crate::preferences::DEFAULT_FONT_LABEL;
use crate::preferences::setting::{Setting, SettingValue};

/// 行高。
pub(super) const ROW: f64 = 22.0;

/// NSFontManager 的常规字重（0–15 档，5 是 regular）。
const REGULAR_WEIGHT: NSInteger = 5;

define_class!(
    // SAFETY: 仅在主线程访问 AppKit 控件；回调里只读自己的快照。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = PickerState]
    pub(super) struct FontListSource;

    unsafe impl NSObjectProtocol for FontListSource {}
    unsafe impl NSControlTextEditingDelegate for FontListSource {}
    unsafe impl NSTableViewDataSource for FontListSource {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> NSInteger {
            self.ivars().visible.borrow().len() as NSInteger
        }
    }
    unsafe impl NSTableViewDelegate for FontListSource {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn view_for_row(
            &self,
            _table: &NSTableView,
            column: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Option<Retained<NSView>> {
            self.cell_view(column, row)
        }

        #[unsafe(method(tableViewSelectionDidChange:))]
        fn selection_changed(&self, _notification: &NSNotification) {
            if self.ivars().syncing.get() {
                return;
            }
            let Some(name) = self.selected_name() else {
                return;
            };
            self.close();
            self.show_current(&name);
            crate::host::with(|h| h.change_setting(self.ivars().setting, SettingValue::Text(name)));
        }
    }
    impl FontListSource {
        /// 搜索框每敲一个字（输入法上屏也算）都来一次：按内容过滤。
        #[unsafe(method(filterChanged:))]
        fn filter_changed(&self, sender: &NSSearchField) {
            let keep = self.selected_name();
            self.apply_filter(&sender.stringValue().to_string());
            self.select(keep.as_deref());
        }
    }
);

impl FontListSource {
    pub(super) fn new(
        mtm: MainThreadMarker,
        families: Vec<String>,
        setting: Setting,
    ) -> Retained<Self> {
        let mut visible = vec![DEFAULT_FONT_LABEL.to_owned()];
        visible.extend(families.iter().cloned());
        let this = mtm.alloc::<Self>().set_ivars(PickerState {
            setting,
            families,
            visible: RefCell::new(visible),
            current: RefCell::new(String::new()),
            button: RefCell::new(None),
            search: RefCell::new(None),
            table: RefCell::new(None),
            popover: RefCell::new(None),
            syncing: Cell::new(false),
        });
        unsafe { msg_send![super(this), init] }
    }

    pub(super) fn attach(
        &self,
        button: Retained<NSPopUpButton>,
        search: Retained<NSSearchField>,
        table: Retained<NSTableView>,
        popover: Retained<NSPopover>,
    ) {
        *self.ivars().button.borrow_mut() = Some(button);
        *self.ivars().search.borrow_mut() = Some(search);
        *self.ivars().table.borrow_mut() = Some(table);
        *self.ivars().popover.borrow_mut() = Some(popover);
    }

    /// 配置变了：记下当前字体，搜索框显示它的名字，列表选中它。
    pub(super) fn set_current(&self, font: &str) {
        *self.ivars().current.borrow_mut() = font.trim().to_owned();
        self.show_current(font);
        self.select((!font.trim().is_empty()).then_some(font.trim()));
    }

    /// 用户点了按钮：清空过滤、选中当前字体、弹出列表、焦点给搜索框。
    pub(super) fn open(&self) {
        let state = self.ivars();
        let (Some(button), Some(search), Some(popover)) = (
            state.button.borrow().clone(),
            state.search.borrow().clone(),
            state.popover.borrow().clone(),
        ) else {
            return;
        };
        if popover.isShown() {
            return;
        }
        search.setStringValue(&NSString::from_str(""));
        self.apply_filter("");
        let current = state.current.borrow().clone();
        self.select((!current.is_empty()).then_some(current.as_str()));
        popover.showRelativeToRect_ofView_preferredEdge(button.bounds(), &button, NSRectEdge::MaxY);
        if let Some(window) = search.window() {
            window.makeFirstResponder(Some(&search));
        }
    }

    fn close(&self) {
        if let Some(popover) = self.ivars().popover.borrow().as_ref()
            && popover.isShown()
        {
            popover.close();
        }
    }

    /// 按钮上显示当前字体名（空为「系统默认」）：按钮的菜单只有这一项，只当显示用。
    fn show_current(&self, font: &str) {
        let font = font.trim();
        let label = if font.is_empty() {
            DEFAULT_FONT_LABEL
        } else {
            font
        };
        if let Some(button) = self.ivars().button.borrow().as_ref() {
            button.removeAllItems();
            button.addItemWithTitle(&NSString::from_str(label));
            button.selectItemAtIndex(0);
        }
    }

    /// 选中某个字族（`None` 或没在列表里就选「系统默认」），不触发写配置。
    fn select(&self, name: Option<&str>) {
        let state = self.ivars();
        let Some(table) = state.table.borrow().clone() else {
            return;
        };
        let row = name
            .and_then(|name| {
                state
                    .visible
                    .borrow()
                    .iter()
                    .position(|f| f.eq_ignore_ascii_case(name))
            })
            .unwrap_or(0);
        state.syncing.set(true);
        table.selectRowIndexes_byExtendingSelection(&NSIndexSet::indexSetWithIndex(row), false);
        table.scrollRowToVisible(row as NSInteger);
        state.syncing.set(false);
    }

    /// 大小写不敏感的子串过滤；「系统默认」永远在第 0 行。
    fn apply_filter(&self, text: &str) {
        let state = self.ivars();
        let needle = text.trim().to_lowercase();
        let mut visible = vec![DEFAULT_FONT_LABEL.to_owned()];
        visible.extend(
            state
                .families
                .iter()
                .filter(|f| needle.is_empty() || f.to_lowercase().contains(&needle))
                .cloned(),
        );
        *state.visible.borrow_mut() = visible;
        if let Some(table) = state.table.borrow().as_ref() {
            state.syncing.set(true);
            table.reloadData();
            state.syncing.set(false);
        }
    }

    fn selected_name(&self) -> Option<String> {
        let state = self.ivars();
        let table = state.table.borrow();
        let row = usize::try_from(table.as_ref()?.selectedRow()).ok()?;
        state.visible.borrow().get(row).cloned()
    }

    fn cell_view(
        &self,
        column: Option<&NSTableColumn>,
        row: NSInteger,
    ) -> Option<Retained<NSView>> {
        let name = self
            .ivars()
            .visible
            .borrow()
            .get(usize::try_from(row).ok()?)?
            .clone();
        let column = column?;
        let mtm = MainThreadMarker::from(self);
        let view = NSView::initWithFrame(
            mtm.alloc(),
            NSRect::new(NSPoint::ZERO, NSSize::new(column.width(), ROW)),
        );
        let text = NSTextField::labelWithString(&NSString::from_str(&name), mtm);
        text.setFrame(NSRect::new(
            NSPoint::new(5.0, 2.0),
            NSSize::new(column.width() - 10.0, ROW - 4.0),
        ));
        // 每行用自己的字体画名字，一眼能看出长什么样；「系统默认」与拿不到字体的用系统字体
        let font = (name != DEFAULT_FONT_LABEL)
            .then(|| {
                NSFontManager::sharedFontManager(mtm).fontWithFamily_traits_weight_size(
                    &NSString::from_str(&name),
                    NSFontTraitMask::empty(),
                    REGULAR_WEIGHT,
                    13.0,
                )
            })
            .flatten()
            .unwrap_or_else(|| NSFont::systemFontOfSize(13.0));
        text.setFont(Some(&font));
        text.setUsesSingleLineMode(true);
        text.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        view.addSubview(&text);
        Some(view)
    }
}
