//! 成对符号（#928）的文档编辑：上屏后光标退回、跳过补上的右半边。
//! 光标后是什么只在这里读得到：跳过先核对，对不上就照常插入。
//! 没有「退格删一对」：Win11 记事本这类应用在没组句时不把退格交给输入法，收不到这个键。
//! 全在上屏那次读写会话里做；括号、引号都是 BMP 字符，一个字就是一个 UTF-16 单元。

use std::mem::ManuallyDrop;

use windows::Win32::UI::TextServices::{
    ITfContext, ITfRange, TF_AE_END, TF_ANCHOR_END, TF_ANCHOR_START, TF_DEFAULT_SELECTION,
    TF_SELECTION, TF_SELECTIONSTYLE,
};
use windows::core::Result;

/// 当前选区折成插入点（终点）；没有选区时为 `None`。
fn caret(context: &ITfContext, ec: u32) -> Option<ITfRange> {
    let mut selection = [TF_SELECTION::default()];
    let mut fetched = 0u32;
    unsafe {
        context
            .GetSelection(ec, TF_DEFAULT_SELECTION, &mut selection, &mut fetched)
            .ok()?;
    }
    if fetched == 0 {
        return None;
    }
    // GetSelection 移交 range 的所有权（ManuallyDrop），取出后由这里释放。
    let range = unsafe { ManuallyDrop::take(&mut selection[0].range) }?;
    unsafe { range.Collapse(ec, TF_ANCHOR_END) }.ok()?;
    Some(range)
}

/// 插入点后的一个字；读不到（文档边界、应用不给读）为 `None`。
fn next_char(range: &ITfRange, ec: u32) -> Option<char> {
    let probe = unsafe { range.Clone() }.ok()?;
    let mut shifted = 0i32;
    unsafe { probe.ShiftEnd(ec, 1, &mut shifted, std::ptr::null()) }.ok()?;
    if shifted != 1 {
        return None;
    }
    let mut buf = [0u16; 1];
    let mut fetched = 0u32;
    unsafe { probe.GetText(ec, 0, &mut buf, &mut fetched) }.ok()?;
    char::from_u32(u32::from(buf[0])).filter(|_| fetched == 1)
}

/// 把插入点挪 `delta` 个字（负数往回）。
pub(super) fn move_caret(context: &ITfContext, ec: u32, delta: i32) -> Result<()> {
    let Some(range) = caret(context, ec) else {
        return Ok(());
    };
    let mut shifted = 0i32;
    unsafe {
        if delta < 0 {
            range.ShiftStart(ec, delta, &mut shifted, std::ptr::null())?;
            range.Collapse(ec, TF_ANCHOR_START)?;
        } else {
            range.ShiftEnd(ec, delta, &mut shifted, std::ptr::null())?;
            range.Collapse(ec, TF_ANCHOR_END)?;
        }
    }
    let selection = TF_SELECTION {
        range: ManuallyDrop::new(Some(range)),
        style: TF_SELECTIONSTYLE {
            ase: TF_AE_END,
            fInterimChar: false.into(),
        },
    };
    // SetSelection 不接管 range 的所有权，之后手动释放。
    let result = unsafe { context.SetSelection(ec, std::slice::from_ref(&selection)) };
    drop(ManuallyDrop::into_inner(selection.range));
    result
}

/// 光标后面正是 `closer` 吗。
pub(super) fn next_is(context: &ITfContext, ec: u32, closer: char) -> bool {
    caret(context, ec).is_some_and(|range| next_char(&range, ec) == Some(closer))
}
