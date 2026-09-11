//! 组句 preedit：把 Server 回来的拼音行经 TSF 组句（[`ITfComposition`]）显示在文档光标处，对应 macOS 的内联 marked text。
//! 这里只放最朴素的一行拼音（不含被纠错划掉的原字母）；富样式的拼音行在候选窗口里另画。
//! 所有写操作都在异步读写编辑会话里做（理由见 `edit_session`）。

mod caret;
mod shared;
mod sink;

use std::mem::ManuallyDrop;
use std::rc::Rc;

use windows::Win32::UI::TextServices::{
    INSERT_TEXT_AT_SELECTION_FLAGS, ITfComposition, ITfCompositionSink, ITfContext,
    ITfContextComposition, ITfInsertAtSelection, ITfRange, TF_AE_END, TF_ANCHOR_END,
    TF_IAS_QUERYONLY, TF_SELECTION, TF_SELECTIONSTYLE,
};
use windows::core::{Interface, Result};

use qingjian_platform::protocol::{Frame, PreeditKind};

pub(crate) use self::shared::Shared;
use self::sink::CompositionSink;

/// 从一帧里拼出内联要显示的拼音行，跳过被纠错划掉的原字母。空串表示没有组句内容。
pub(crate) fn preedit_string(frame: &Frame) -> String {
    frame
        .preedit
        .iter()
        .filter(|segment| segment.kind != PreeditKind::Corrected)
        .map(|segment| segment.text.as_str())
        .collect()
}

/// 把文档更新到本次按键算出的目标状态：先落定 `commit`，再按 `preedit` 起 / 改 / 收组句，最后摆候选窗口。
/// 在编辑会话回调（持写锁 `ec`）里调。`shared` 用 `&Rc` 是因为起新组句要把它克隆进 [`CompositionSink`]。
pub(crate) fn apply(
    shared: &Rc<Shared>,
    context: &ITfContext,
    ec: u32,
    commit: Option<&str>,
    preedit: &str,
) -> Result<()> {
    if let Some(text) = commit {
        commit_text(shared, context, ec, text)?;
    }
    if preedit.is_empty() {
        end_composition(shared, ec)?;
    } else {
        update_preedit(shared, context, ec, preedit)?;
    }
    // 此刻有 ec 和组句范围，能拿到光标屏幕矩形。
    update_window(shared, context, ec);
    Ok(())
}

/// 组句在进行就把候选窗口摆到光标下方，否则收起。内容已在收键时设好。
fn update_window(shared: &Shared, context: &ITfContext, ec: u32) {
    let active = shared.composition();
    shared.with_window(|window| match &active {
        Some(composition) => {
            let anchor =
                caret::caret_rect(context, ec, composition).unwrap_or_else(caret::mouse_anchor);
            window.show(anchor);
        }
        None => window.hide(),
    });
}

/// 落定上屏文本：有组句就把组句范围替换成它再结束组句，否则在选区插入。
fn commit_text(shared: &Shared, context: &ITfContext, ec: u32, text: &str) -> Result<()> {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    match shared.composition() {
        Some(composition) => {
            // SAFETY: ec 是本会话写锁；composition 是我们起的活动组句。
            let range = unsafe { composition.GetRange()? };
            unsafe { range.SetText(ec, 0, &utf16)? };
            move_selection_to_end(context, ec, &range)?;
            unsafe { composition.EndComposition(ec)? };
            shared.set_composition(None);
        }
        None => {
            let insert: ITfInsertAtSelection = context.cast()?;
            // SAFETY: ec 有效。标志用 0 不用 NOQUERY：NOQUERY 不回传 range，windows-rs 会把 NULL 当失败。
            unsafe {
                insert.InsertTextAtSelection(ec, INSERT_TEXT_AT_SELECTION_FLAGS(0), &utf16)?;
            }
        }
    }
    Ok(())
}

/// 把组句拼音行更新成 `preedit`：没有活动组句就在选区处起一个，整段替换文本并把光标移到末尾。
fn update_preedit(shared: &Rc<Shared>, context: &ITfContext, ec: u32, preedit: &str) -> Result<()> {
    let composition = match shared.composition() {
        Some(composition) => composition,
        None => start_composition(shared, context, ec)?,
    };
    let utf16: Vec<u16> = preedit.encode_utf16().collect();
    // SAFETY: ec 是写锁；composition 活动中。
    let range = unsafe { composition.GetRange()? };
    unsafe { range.SetText(ec, 0, &utf16)? };
    move_selection_to_end(context, ec, &range)
}

/// 在当前选区处起一个空组句并存起来；组句 sink 随组句交给框架持有。
fn start_composition(shared: &Rc<Shared>, context: &ITfContext, ec: u32) -> Result<ITfComposition> {
    let insert: ITfInsertAtSelection = context.cast()?;
    // SAFETY: ec 有效；QUERYONLY 不插入，只取选区处的空范围当组句起点。
    let range = unsafe { insert.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])? };
    let context_composition: ITfContextComposition = context.cast()?;
    let sink: ITfCompositionSink = CompositionSink::new(shared.clone()).into();
    // SAFETY: ec 是写锁；range 是刚取到的选区范围；sink 由框架 AddRef 持有。
    let composition = unsafe { context_composition.StartComposition(ec, &range, &sink)? };
    shared.set_composition(Some(composition.clone()));
    Ok(composition)
}

/// 收掉组句（若有）：清空组句文本再结束，避免残留拼音。
fn end_composition(shared: &Shared, ec: u32) -> Result<()> {
    if let Some(composition) = shared.take_composition() {
        // SAFETY: ec 是写锁；composition 是我们起的活动组句。
        let range = unsafe { composition.GetRange()? };
        unsafe { range.SetText(ec, 0, &[])? };
        unsafe { composition.EndComposition(ec)? };
    }
    Ok(())
}

/// 把选区折叠到 `range` 末尾。
fn move_selection_to_end(context: &ITfContext, ec: u32, range: &ITfRange) -> Result<()> {
    // SAFETY: ec 是写锁；range 属于本上下文。
    let end = unsafe { range.Clone()? };
    unsafe { end.Collapse(ec, TF_ANCHOR_END)? };
    let selection = TF_SELECTION {
        range: ManuallyDrop::new(Some(end)),
        style: TF_SELECTIONSTYLE {
            ase: TF_AE_END,
            fInterimChar: false.into(),
        },
    };
    // SAFETY: ec 是写锁；SetSelection 只读这个数组、不接管所有权，所以之后要手动释放 range。
    let result = unsafe { context.SetSelection(ec, std::slice::from_ref(&selection)) };
    drop(ManuallyDrop::into_inner(selection.range));
    result
}
