//! 中 / 英 输入模式指示器：驱动 TSF 的「转换模式」compartment，系统任务栏据此显示「中」或「英」。
//!
//! 中文模式点亮 `TF_CONVERSIONMODE_NATIVE` 位，英文模式清掉，其它位（全 / 半角等）保留。写不进去只记日志，
//! 指示器不动不影响打字。暂时是单向：Shift 切换时我们写；用户点任务栏改模式还不会反过来同步 DLL 状态（待办）。

use windows::Win32::System::Variant::VT_I4;
use windows::Win32::UI::TextServices::{
    GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION, ITfCompartment, ITfCompartmentMgr,
    ITfThreadMgr, TF_CONVERSIONMODE_NATIVE,
};
use windows::core::{Interface, Result};

use super::log::log;
use super::variant::i4;

/// 把系统的中 / 英指示器设成当前模式。失败只记日志。
pub(super) fn set_indicator(thread_mgr: &ITfThreadMgr, tid: u32, english: bool) {
    if let Err(error) = write_conversion_mode(thread_mgr, tid, english) {
        log(&format!("设置中英指示器失败: {error}"));
    }
}

fn write_conversion_mode(thread_mgr: &ITfThreadMgr, tid: u32, english: bool) -> Result<()> {
    let mgr: ITfCompartmentMgr = thread_mgr.cast()?;
    // SAFETY: GUID 指针有效。
    let compartment =
        unsafe { mgr.GetCompartment(&GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION)? };
    let native = TF_CONVERSIONMODE_NATIVE as i32;
    // 读不到（没设过）按中文（NATIVE 亮）起算，只翻这一位、别动全 / 半角。
    let current = read_i4(&compartment).unwrap_or(native);
    let next = if english {
        current & !native
    } else {
        current | native
    };
    let variant = i4(next);
    // SAFETY: variant 是本地 VT_I4，SetValue 只读它。
    unsafe { compartment.SetValue(tid, &variant) }
}

/// 读 compartment 里的 `VT_I4` 值；类型不对或读不到返回 `None`。
fn read_i4(compartment: &ITfCompartment) -> Option<i32> {
    // SAFETY: GetValue 返回一个随即读取的 VARIANT。
    let variant = unsafe { compartment.GetValue().ok()? };
    // SAFETY: 读联合体前先核对 vt 是 VT_I4。
    unsafe {
        let inner = &variant.Anonymous.Anonymous;
        (inner.vt == VT_I4).then(|| inner.Anonymous.lVal)
    }
}
