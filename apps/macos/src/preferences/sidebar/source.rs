//! 侧栏表格的数据源与代理：每行一个「图标 + 页名」单元，选中变化时回调页号。

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSControlTextEditingDelegate, NSFont, NSImage, NSImageView, NSTableCellView, NSTableColumn,
    NSTableView, NSTableViewDataSource, NSTableViewDelegate, NSTextField, NSView,
};
use objc2_foundation::{
    NSInteger, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use super::SidebarEntry;

/// 图标列的宽度（图标居中放在里面）。
const ICON_WIDTH: f64 = 24.0;

/// 图标离单元左边的距离。
const ICON_X: f64 = 6.0;

/// 页名离单元左边的距离。
const TITLE_X: f64 = ICON_X + ICON_WIDTH + 6.0;

/// 选中变化的回调。
type OnSelect = Box<dyn Fn(usize)>;

/// 侧栏数据源的状态。
pub struct Ivars {
    /// 各行的页名与图标。
    entries: Vec<SidebarEntry>,

    /// 选中变化的回调。
    on_select: RefCell<Option<OnSelect>>,
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    /// 侧栏表格的数据源与代理。
    pub struct SidebarSource;

    unsafe impl NSObjectProtocol for SidebarSource {}
    unsafe impl NSControlTextEditingDelegate for SidebarSource {}

    unsafe impl NSTableViewDataSource for SidebarSource {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> NSInteger {
            self.ivars().entries.len() as NSInteger
        }
    }

    unsafe impl NSTableViewDelegate for SidebarSource {
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
        fn selection_changed(&self, notification: &NSNotification) {
            // 通知的 object 就是表格；不用另存表格的引用
            let Some(table) = notification.object() else {
                return;
            };
            let Ok(table) = table.downcast::<NSTableView>() else {
                return;
            };
            let Ok(row) = usize::try_from(table.selectedRow()) else {
                return;
            };
            if let Some(on_select) = self.ivars().on_select.borrow().as_ref() {
                on_select(row);
            }
        }
    }
);

impl SidebarSource {
    pub fn new(
        mtm: MainThreadMarker,
        entries: Vec<SidebarEntry>,
        on_select: OnSelect,
    ) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(Ivars {
            entries,
            on_select: RefCell::new(Some(on_select)),
        });
        unsafe { msg_send![super(this), init] }
    }

    /// 一行的单元：NSTableCellView 挂上 imageView / textField，选中时 AppKit 会自动把文字换成高亮色。
    fn cell_view(
        &self,
        column: Option<&NSTableColumn>,
        row: NSInteger,
    ) -> Option<Retained<NSView>> {
        let entry = self.ivars().entries.get(usize::try_from(row).ok()?)?;
        let width = column?.width();
        let height = super::ROW_HEIGHT;
        let mtm = MainThreadMarker::from(self);
        let cell = NSTableCellView::initWithFrame(
            mtm.alloc(),
            NSRect::new(NSPoint::ZERO, NSSize::new(width, height)),
        );
        // 名字是常量里的合法 SF Symbol；macOS 11+ 才有此 API，最低系统是 13
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str(entry.symbol),
            None,
        );
        let icon = NSImageView::initWithFrame(
            mtm.alloc(),
            NSRect::new(
                NSPoint::new(ICON_X, (height - ICON_WIDTH) / 2.0),
                NSSize::new(ICON_WIDTH, ICON_WIDTH),
            ),
        );
        icon.setImage(image.as_deref());
        let title = NSTextField::labelWithString(&NSString::from_str(entry.title), mtm);
        title.setFont(Some(&NSFont::systemFontOfSize(13.0)));
        title.setFrame(NSRect::new(
            NSPoint::new(TITLE_X, (height - 17.0) / 2.0),
            NSSize::new(width - TITLE_X - 8.0, 17.0),
        ));
        cell.addSubview(&icon);
        cell.addSubview(&title);
        // SAFETY: 两个视图已是单元的子视图，单元只弱引用它们
        unsafe {
            cell.setImageView(Some(&icon));
            cell.setTextField(Some(&title));
        }
        Some(Retained::into_super(cell))
    }
}
