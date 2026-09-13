use super::state::TableState;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::{
    NSButton, NSColor, NSControlStateValueOn, NSControlTextEditingDelegate, NSFont,
    NSLineBreakMode, NSTableColumn, NSTableView, NSTableViewDataSource, NSTableViewDelegate,
    NSTextAlignment, NSTextField, NSView,
};
use objc2_foundation::{
    NSInteger, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use qingjian_core::CustomPhrase;
use std::cell::RefCell;

define_class!(
    // SAFETY: 仅在主线程访问 AppKit 控件，数据回调不访问 Host。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = TableState]
    pub(super) struct PhraseTableSource;

    unsafe impl NSObjectProtocol for PhraseTableSource {}
    unsafe impl NSControlTextEditingDelegate for PhraseTableSource {}
    unsafe impl NSTableViewDataSource for PhraseTableSource {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> NSInteger {
            self.ivars().rows.borrow().len() as NSInteger
        }
    }
    unsafe impl NSTableViewDelegate for PhraseTableSource {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn view_for_row(&self, _table: &NSTableView, column: Option<&NSTableColumn>, row: NSInteger) -> Option<Retained<NSView>> {
            (|| {
            let rows = self.ivars().rows.borrow();
            let phrase = rows.get(usize::try_from(row).ok()?)?;
            let column = column?;
            let mtm = MainThreadMarker::from(self);
            let view = NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::ZERO, NSSize::new(column.width(), 28.0)));
            let identifier = column.identifier().to_string();
            if identifier == "enabled" {
                let check = unsafe { NSButton::checkboxWithTitle_target_action(&NSString::from_str(""), Some(self), Some(sel!(togglePhrase:)), mtm) };
                check.setFrame(NSRect::new(NSPoint::new(18.0, 4.0), NSSize::new(22.0, 20.0)));
                check.setState(if phrase.enabled { NSControlStateValueOn } else { 0 });
                check.setTag(row);
                view.addSubview(&check);
            } else {
                let value = if identifier == "position" { phrase.position.to_string() } else { preview(phrase) };
                let text = NSTextField::labelWithString(&NSString::from_str(&value), mtm);
                text.setFrame(NSRect::new(NSPoint::new(5.0, 4.0), NSSize::new(column.width()-10.0, 20.0)));
                text.setFont(Some(&NSFont::systemFontOfSize(13.0)));
                text.setUsesSingleLineMode(true);
                text.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
                if identifier == "position" { text.setAlignment(NSTextAlignment::Center); }
                if !phrase.enabled { text.setTextColor(Some(&NSColor::secondaryLabelColor())); }
                view.addSubview(&text);
            }
            Some(view)
            })()
        }
        #[unsafe(method(tableViewSelectionDidChange:))]
        fn selection_changed(&self, _notification: &NSNotification) {
            let selected = self.ivars().table.borrow().as_ref().is_some_and(|t| t.selectedRow() >= 0);
            self.ivars().edit.setEnabled(selected);
            self.ivars().delete.setEnabled(selected);
        }
    }
    impl PhraseTableSource {
        #[unsafe(method(togglePhrase:))]
        fn toggle_phrase(&self, sender: &NSButton) {
            let Ok(index) = usize::try_from(sender.tag()) else { return; };
            crate::host::with(|h| h.set_phrase_enabled(index, sender.state() == NSControlStateValueOn));
        }
    }
);
impl PhraseTableSource {
    pub fn new(
        mtm: MainThreadMarker,
        edit: Retained<NSButton>,
        delete: Retained<NSButton>,
    ) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(TableState {
            rows: RefCell::new(Vec::new()),
            table: RefCell::new(None),
            edit,
            delete,
        });
        unsafe { msg_send![super(this), init] }
    }
    pub fn attach(&self, table: Retained<NSTableView>) {
        *self.ivars().table.borrow_mut() = Some(table);
    }
    pub fn replace(&self, rows: &[CustomPhrase]) {
        *self.ivars().rows.borrow_mut() = rows.to_vec();
    }
}
fn preview(phrase: &CustomPhrase) -> String {
    let mut chars = phrase.text.chars();
    let mut text: String = chars
        .by_ref()
        .take(80)
        .map(|c| match c {
            '\n' | '\r' => '↵',
            '\t' => '⇥',
            _ => c,
        })
        .collect();
    if chars.next().is_some() {
        text.push('…');
    }
    format!("{} = {}", phrase.code, text)
}
