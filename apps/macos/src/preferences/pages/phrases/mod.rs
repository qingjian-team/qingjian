//! 自定义文本表格与独立编辑表单。
mod state;
mod table;
use crate::preferences::{
    controls::{button, checkbox, note_full, row_popup, select, set_checked, text_field},
    layout::{Layout, PAGE_PADDING, ROW_HEIGHT},
    setting::Setting,
    target::PreferencesTarget,
};
use objc2::{MainThreadMarker, rc::Retained, runtime::ProtocolObject, sel};
use objc2_app_kit::{
    NSBackingStoreType, NSBorderType, NSButton, NSColor, NSControlStateValueOn, NSPopUpButton,
    NSScrollView, NSTableColumn, NSTableView, NSTextField, NSTextView, NSView, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSIndexSet, NSPoint, NSRect, NSSize, NSString};
use qingjian_core::CustomPhrase;
use qingjian_platform::Config;
use std::cell::Cell;
use table::PhraseTableSource;

pub struct PhrasesPage {
    /// 常驻规则列表。
    table: Retained<NSTableView>,

    /// 表格数据源与代理。
    _source: Retained<PhraseTableSource>,

    /// 新增或编辑时才显示的表单。
    editor: Retained<NSWindow>,

    /// 表单校验错误，不清除草稿。
    error: Retained<NSTextField>,

    /// 输入码。
    code: Retained<NSTextField>,

    /// 位置。
    position: Retained<NSPopUpButton>,

    /// 多行文本。
    text: Retained<NSTextView>,

    /// 启用状态。
    enabled: Retained<NSButton>,

    /// 编辑中的规则下标；None 为新增。
    selected: Cell<Option<usize>>,
}
impl PhrasesPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let edit = button(mtm, "编辑…", Setting::EditPhrase, target);
        let delete = button(mtm, "−", Setting::DeletePhrase, target);
        edit.setEnabled(false);
        delete.setEnabled(false);
        let table = NSTableView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        table.setRowHeight(28.0);
        table.setUsesAlternatingRowBackgroundColors(true);
        table.setAllowsMultipleSelection(false);
        table.setAllowsEmptySelection(true);
        for (id, title, width) in [
            ("enabled", "启用", 58.0),
            ("content", "内容", 326.0),
            ("position", "显示位置", 86.0),
        ] {
            let column = NSTableColumn::initWithIdentifier(mtm.alloc(), &NSString::from_str(id));
            column.setTitle(&NSString::from_str(title));
            column.setWidth(width);
            table.addTableColumn(&column);
        }
        let source = PhraseTableSource::new(mtm, edit.clone(), delete.clone());
        source.attach(table.clone());
        // SAFETY: 数据源保留在页面内，选择器签名与 PreferencesTarget 相符。
        unsafe {
            table.setDataSource(Some(ProtocolObject::from_ref(&*source)));
            table.setDelegate(Some(ProtocolObject::from_ref(&*source)));
            table.setTarget(Some(target));
            table.setDoubleAction(Some(sel!(editPhrase:)));
        }
        let list = NSScrollView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        list.setHasVerticalScroller(true);
        list.setBorderType(NSBorderType::BezelBorder);
        list.setDocumentView(Some(&table));
        layout.place(&list, PAGE_PADDING, layout.inner_width(), 332.0);
        layout.next_row(332.0);
        let add = button(mtm, "+", Setting::NewPhrase, target);
        layout.place(&add, PAGE_PADDING, 34.0, ROW_HEIGHT);
        layout.place(&delete, PAGE_PADDING + 36.0, 34.0, ROW_HEIGHT);
        layout.place(&edit, PAGE_PADDING + 84.0, 90.0, ROW_HEIGHT);
        layout.next_row(ROW_HEIGHT);
        note_full(
            layout,
            mtm,
            "勾选启用，双击一行编辑。长文本仅在列表中缩略显示。 ",
        );
        let mut form = Layout::new(520.0, 18.0);
        let layout = &mut form;
        note_full(
            layout,
            mtm,
            "输入码：1–32 个小写字母。相同输入码可设置多个不同位置；位置冲突时不能保存。 ",
        );
        let code = text_field(mtm, Setting::PhraseDraft, target);
        code.setPlaceholderString(Some(&NSString::from_str("输入码，例如 ee")));
        layout.place(&code, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT);
        layout.next_row(ROW_HEIGHT);
        let position = row_popup(
            layout,
            mtm,
            "固定候选位置",
            &(1..=9).map(|n| n.to_string()).collect::<Vec<_>>(),
            Setting::PhraseDraft,
            target,
        );
        let enabled = checkbox(mtm, "启用这条规则", Setting::PhraseDraft, target);
        layout.place(&enabled, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT);
        layout.next_row(ROW_HEIGHT);
        note_full(
            layout,
            mtm,
            "快捷文本（保留空格与换行，可输入地址、长文本、符号）：",
        );
        let scroll = NSScrollView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        scroll.setHasVerticalScroller(true);
        let text = NSTextView::initWithFrame(
            mtm.alloc(),
            NSRect::new(
                objc2_foundation::NSPoint::ZERO,
                objc2_foundation::NSSize::new(layout.inner_width(), 180.0),
            ),
        );
        text.setRichText(false);
        text.setVerticallyResizable(true);
        text.setHorizontallyResizable(false);
        text.setAutomaticQuoteSubstitutionEnabled(false);
        text.setAutomaticDashSubstitutionEnabled(false);
        scroll.setDocumentView(Some(&text));
        layout.place(&scroll, PAGE_PADDING, layout.inner_width(), 180.0);
        layout.next_row(180.0);
        let save = button(mtm, "保存规则", Setting::SavePhrase, target);
        let cancel = button(mtm, "取消", Setting::CancelPhraseEdit, target);
        cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
        layout.place(&save, PAGE_PADDING, 130.0, ROW_HEIGHT);
        layout.place(&cancel, PAGE_PADDING + 145.0, 100.0, ROW_HEIGHT);
        layout.next_row(ROW_HEIGHT);
        let error = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        error.setTextColor(Some(&NSColor::systemRedColor()));
        layout.place(&error, PAGE_PADDING, layout.inner_width(), 32.0);
        layout.next_row(32.0);
        let height = form.height() + 18.0;
        let content = NSView::initWithFrame(
            mtm.alloc(),
            NSRect::new(NSPoint::ZERO, NSSize::new(520.0, height)),
        );
        form.finish(&content, height);
        let editor = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                NSRect::new(NSPoint::ZERO, NSSize::new(520.0, height)),
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe {
            editor.setReleasedWhenClosed(false);
        }
        editor.setContentView(Some(&content));
        Self {
            table,
            _source: source,
            editor,
            error,
            code,
            position,
            text,
            enabled,
            selected: Cell::new(None),
        }
    }
    pub fn sync(&self, config: &Config) {
        let selected = self
            .selected_row()
            .filter(|&i| i < config.custom_phrases.len());
        self._source.replace(&config.custom_phrases);
        self.table.reloadData();
        self.select_row(selected);
    }
    pub fn selected_row(&self) -> Option<usize> {
        usize::try_from(self.table.selectedRow()).ok()
    }
    pub fn select_row(&self, index: Option<usize>) {
        if let Some(index) = index {
            self.table.selectRowIndexes_byExtendingSelection(
                &NSIndexSet::indexSetWithIndex(index),
                false,
            );
        } else {
            unsafe {
                self.table.deselectAll(None);
            }
        }
    }
    pub fn load(&self, config: &Config, index: usize) {
        self.select_row(
            index
                .checked_sub(1)
                .filter(|&i| i < config.custom_phrases.len()),
        );
    }
    pub fn edit(&self, config: &Config, index: Option<usize>) {
        self.selected.set(index);
        let p = index.and_then(|i| config.custom_phrases.get(i));
        self.code
            .setStringValue(&NSString::from_str(p.map_or("", |p| &p.code)));
        self.text
            .setString(&NSString::from_str(p.map_or("", |p| &p.text)));
        select(&self.position, Some(p.map_or(0, |p| p.position - 1)));
        set_checked(&self.enabled, p.is_none_or(|p| p.enabled));
        self.error.setStringValue(&NSString::from_str(""));
        self.editor
            .setTitle(&NSString::from_str(if index.is_some() {
                "编辑快捷文本"
            } else {
                "新增快捷文本"
            }));
        if let Some(parent) = self.table.window() {
            parent.beginSheet_completionHandler(&self.editor, None);
            self.editor.makeFirstResponder(Some(&self.code));
        }
    }
    pub fn close_editor(&self) {
        if let Some(parent) = self.editor.sheetParent() {
            parent.endSheet(&self.editor);
        }
        self.editor.orderOut(None);
    }
    pub fn set_error(&self, error: &str) {
        self.error.setStringValue(&NSString::from_str(error));
    }
    pub fn selected(&self) -> Option<usize> {
        self.selected.get()
    }
    pub fn draft(&self) -> CustomPhrase {
        CustomPhrase {
            code: self.code.stringValue().to_string(),
            text: self.text.string().to_string(),
            position: self.position.indexOfSelectedItem() as usize + 1,
            enabled: self.enabled.state() == NSControlStateValueOn,
        }
    }
}
