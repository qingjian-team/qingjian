//! 偏好设置各页共用的控件搭建函数：标签、说明小字、成行的弹出菜单 / 文本框 / 录制按钮，
//! 以及把控件接到 [`PreferencesTarget`] 的 `changed:` 上。页面文件只描述「放什么」，不重复这些细节。

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSButton, NSColor, NSControl, NSControlStateValueOff, NSControlStateValueOn, NSFont,
    NSPopUpButton, NSSecureTextField, NSTextAlignment, NSTextField,
};
use objc2_foundation::{NSArray, NSRect, NSString};
use qingjian_core::Language;

use super::key_recorder::KeyRecorder;
use super::layout::{CONTROL_X, LABEL_WIDTH, Layout, PAGE_PADDING, ROW_HEIGHT};
use super::setting::Setting;
use super::target::PreferencesTarget;

/// 说明小字一行的高度。
pub(super) const NOTE_HEIGHT: f64 = 15.0;

/// 说明小字按多少像素一个字估算折行（11 号字，中文约 11 px，估得宽一点宁可多留一行）。
const NOTE_CHAR_WIDTH: f64 = 11.5;

/// 分组之间的留白。
pub(super) const GROUP_GAP: f64 = 14.0;

pub(super) fn language_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "中文",
        Language::English => "英语",
        Language::Japanese => "日语",
    }
}

/// `,.` → `,  .`。
pub(super) fn page_keys_label(pair: &str) -> String {
    let mut chars = pair.chars();
    match (chars.next(), chars.next()) {
        (Some(previous), Some(next)) => format!("{previous}   {next}"),
        _ => pair.to_owned(),
    }
}

pub(super) fn select(popup: &NSPopUpButton, index: Option<usize>) {
    if let Some(index) = index {
        popup.selectItemAtIndex(index as isize);
    }
}

pub(super) fn set_checked(button: &NSButton, on: bool) {
    button.setState(if on {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
}

/// 挂 target / action / tag：所有控件都发 `changed:`，靠 tag 区分。
fn wire(control: &NSControl, setting: Setting, target: &PreferencesTarget) {
    // SAFETY: 选择器与 PreferencesTarget 上定义的 `changed:` 一致，签名 (id) -> void
    unsafe {
        control.setTarget(Some(target));
        control.setAction(Some(sel!(changed:)));
    }
    control.setTag(setting.tag());
}

pub(super) fn small_label(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFont(Some(&NSFont::systemFontOfSize(11.0)));
    label.setTextColor(Some(&NSColor::secondaryLabelColor()));
    label
}

/// 控件下方的说明小字，与控件列对齐，放不下就折行（按字数估行数，宁可多留一行）。
pub(super) fn note(layout: &mut Layout, mtm: MainThreadMarker, text: &str) {
    note_at(layout, mtm, text, CONTROL_X, layout.control_width());
}

/// 整行宽的说明小字（勾选框、按钮下面用）。
pub(super) fn note_full(layout: &mut Layout, mtm: MainThreadMarker, text: &str) {
    note_at(layout, mtm, text, PAGE_PADDING, layout.inner_width());
}

fn note_at(layout: &mut Layout, mtm: MainThreadMarker, text: &str, x: f64, width: f64) {
    let label = small_label(mtm, text);
    label.setUsesSingleLineMode(false);
    if let Some(cell) = label.cell() {
        cell.setWraps(true);
    }
    let estimated = text.chars().count() as f64 * NOTE_CHAR_WIDTH;
    let lines = (estimated / width).ceil().max(1.0);
    let height = NOTE_HEIGHT * lines;
    layout.place(&label, x, width, height);
    layout.next_row(height);
}

/// 勾选框独占一行，从标题列起始处摆（勾选框自带标题，不用左列标题）。
pub(super) fn row_checkbox(layout: &mut Layout, button: &NSButton) {
    layout.place(button, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT);
    layout.next_row(ROW_HEIGHT);
}

/// 标题列，右对齐贴着控件。
pub(super) fn caption(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setAlignment(NSTextAlignment::Right);
    label
}

/// 一行「标题 + 控件」，控件占满控件列。
pub(super) fn row_control(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    title: &str,
    control: &NSControl,
) {
    let label = caption(mtm, title);
    layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
    layout.place(control, CONTROL_X, layout.control_width(), ROW_HEIGHT);
    layout.next_row(ROW_HEIGHT);
}

pub(super) fn row_popup(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    title: &str,
    titles: &[String],
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSPopUpButton> {
    let popup = NSPopUpButton::initWithFrame_pullsDown(mtm.alloc(), NSRect::ZERO, false);
    let items: Vec<Retained<NSString>> = titles.iter().map(|t| NSString::from_str(t)).collect();
    popup.addItemsWithTitles(&NSArray::from_retained_slice(&items));
    wire(&popup, setting, target);
    let label = caption(mtm, title);
    layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
    layout.place(
        &popup,
        CONTROL_X,
        layout.control_width().min(200.0),
        ROW_HEIGHT,
    );
    layout.next_row(ROW_HEIGHT);
    popup
}

/// 一行「标题 + 快捷键录制按钮」。
pub(super) fn row_recorder(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    title: &str,
    setting: Setting,
    modifiers_only: bool,
    target: &PreferencesTarget,
) -> Retained<KeyRecorder> {
    let recorder = KeyRecorder::new(mtm, modifiers_only);
    wire(&recorder, setting, target);
    let label = caption(mtm, title);
    layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
    layout.place(&recorder, CONTROL_X, 160.0, ROW_HEIGHT);
    layout.next_row(ROW_HEIGHT);
    recorder
}

pub(super) fn checkbox(
    mtm: MainThreadMarker,
    title: &str,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSButton> {
    // SAFETY: target / action 与 `wire` 相同
    let button = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(sel!(changed:)),
            mtm,
        )
    };
    button.setTag(setting.tag());
    button
}

pub(super) fn button(
    mtm: MainThreadMarker,
    title: &str,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSButton> {
    // SAFETY: 同上
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(sel!(changed:)),
            mtm,
        )
    };
    button.setTag(setting.tag());
    button
}

/// 可编辑单行文本框：回车或失焦时发 action。
pub(super) fn text_field(
    mtm: MainThreadMarker,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSTextField> {
    let field = NSTextField::initWithFrame(mtm.alloc(), NSRect::ZERO);
    editable(&field, setting, target);
    field
}

pub(super) fn secure_field(
    mtm: MainThreadMarker,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSSecureTextField> {
    let field = NSSecureTextField::initWithFrame(mtm.alloc(), NSRect::ZERO);
    editable(&field, setting, target);
    field
}

fn editable(field: &NSTextField, setting: Setting, target: &PreferencesTarget) {
    field.setBezeled(true);
    field.setEditable(true);
    field.setSelectable(true);
    field.setDrawsBackground(true);
    field.setUsesSingleLineMode(true);
    if let Some(cell) = field.cell() {
        cell.setSendsActionOnEndEditing(true);
    }
    wire(field, setting, target);
}
