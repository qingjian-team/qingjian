//! 组句起始时读应用光标前的文字，给本地整句模型当前文（对应 macOS 壳的 `surrounding_text`）。
//! 在起组句的那次读写会话里顺手读（此时选区还是原来的插入点，拼音还没插进去），不另开会话。

use std::mem::ManuallyDrop;

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Variant::VT_UNKNOWN;
use windows::Win32::UI::TextServices::{
    GUID_PROP_INPUTSCOPE, IS_ALPHANUMERIC_PIN, IS_NUMERIC_PASSWORD, IS_NUMERIC_PIN, IS_PASSWORD,
    IS_PRIVATE, ITfContext, ITfInputScope, ITfRange, InputScope, TF_ANCHOR_START,
    TF_DEFAULT_SELECTION, TF_SELECTION,
};
use windows::core::Interface;

/// 往前读多少字（与 macOS 壳的 `RESCORE_LOOKBACK` 一致）。
const LOOKBACK: i32 = 64;

/// 当前选区起点之前最多 [`LOOKBACK`] 个 UTF-16 单元的文本。密码框、没有选区、读不到时为 `None`。
pub(crate) fn text_before_caret(context: &ITfContext, ec: u32) -> Option<String> {
    let range = selection_start(context, ec)?;
    if is_password(context, ec, &range) {
        crate::com::log::log("密码框，不读光标前文");
        return None;
    }
    let mut shifted = 0i32;
    unsafe { range.ShiftStart(ec, -LOOKBACK, &mut shifted, std::ptr::null()) }.ok()?;
    if shifted == 0 {
        return None;
    }
    let mut buf = [0u16; LOOKBACK as usize];
    let mut fetched = 0u32;
    unsafe { range.GetText(ec, 0, &mut buf, &mut fetched) }.ok()?;
    let text = String::from_utf16_lossy(&buf[..fetched as usize]);
    (!text.is_empty()).then_some(text)
}

/// 选区折成起点（插入点）。
fn selection_start(context: &ITfContext, ec: u32) -> Option<ITfRange> {
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
    unsafe { range.Collapse(ec, TF_ANCHOR_START) }.ok()?;
    Some(range)
}

/// 算作密码、不读前文的输入范围：密码 / PIN 之外还有 `IS_PRIVATE`——Chromium（Edge / Chrome）的密码框
/// 与无痕窗口报的是它而不是 `IS_PASSWORD`。
const SECRET_SCOPES: [InputScope; 5] = [
    IS_PASSWORD,
    IS_PRIVATE,
    IS_NUMERIC_PASSWORD,
    IS_NUMERIC_PIN,
    IS_ALPHANUMERIC_PIN,
];

/// 输入框声明了密码类输入范围（`GUID_PROP_INPUTSCOPE` 里含 [`SECRET_SCOPES`] 之一）。拿不到属性按不是密码。
fn is_password(context: &ITfContext, ec: u32, range: &ITfRange) -> bool {
    match input_scopes(context, ec, range) {
        Ok(scopes) => {
            crate::com::log::log(&format!("输入范围: {scopes:?}"));
            scopes.iter().any(|scope| SECRET_SCOPES.contains(scope))
        }
        // 不支持输入范围属性的应用（如记事本）GetValue 会失败，按不是密码，不记日志
        Err(_) => false,
    }
}

/// 应用给 `range` 声明的全部输入范围；哪一步拿不到就说哪一步。
fn input_scopes(
    context: &ITfContext,
    ec: u32,
    range: &ITfRange,
) -> Result<Vec<InputScope>, String> {
    let property = unsafe { context.GetAppProperty(&GUID_PROP_INPUTSCOPE) }
        .map_err(|error| format!("GetAppProperty {error}"))?;
    let value =
        unsafe { property.GetValue(ec, range) }.map_err(|error| format!("GetValue {error}"))?;
    // SAFETY: 只在 vt 是 VT_UNKNOWN 时读 punkVal 那个联合体成员。
    let scope: ITfInputScope = unsafe {
        let inner = &value.Anonymous.Anonymous;
        if inner.vt != VT_UNKNOWN {
            return Err(format!("vt={}", inner.vt.0));
        }
        inner
            .Anonymous
            .punkVal
            .as_ref()
            .ok_or_else(|| "punkVal 空".to_owned())?
            .cast()
            .map_err(|error| format!("cast ITfInputScope {error}"))?
    };
    let mut scopes: *mut InputScope = std::ptr::null_mut();
    let mut count = 0u32;
    unsafe { scope.GetInputScopes(&mut scopes, &mut count) }
        .map_err(|error| format!("GetInputScopes {error}"))?;
    if scopes.is_null() {
        return Err("GetInputScopes 返回空数组".to_owned());
    }
    // SAFETY: GetInputScopes 返回 count 个元素的 CoTaskMem 数组，由调用方释放。
    unsafe {
        let list = std::slice::from_raw_parts(scopes, count as usize).to_vec();
        CoTaskMemFree(Some(scopes.cast()));
        Ok(list)
    }
}
