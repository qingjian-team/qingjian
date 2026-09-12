//! 文档上下文级的开关：TSF 规定键盘类文本服务**必须**看 `GUID_COMPARTMENT_KEYBOARD_DISABLED`（非零 = 所有键原样放行、不组句），
//! 微软 SampleIME 连同 `GUID_COMPARTMENT_EMPTYCONTEXT` 一起查。密码框走的就是这条：微软文档明说密码框应当禁用文本服务
//! （`IS_PASSWORD` 只是标注、不提供保护），Chromium 系浏览器给密码框的上下文设的也是它。对应 macOS 的 Secure Input：直接不组句。

use windows::Win32::UI::TextServices::{
    GUID_COMPARTMENT_EMPTYCONTEXT, GUID_COMPARTMENT_KEYBOARD_DISABLED, ITfCompartmentMgr,
    ITfContext,
};
use windows::core::{GUID, Interface};

/// 这个上下文禁了键盘输入（密码框、空上下文）：所有键放行。读不到按没禁。
pub(crate) fn keyboard_disabled(context: &ITfContext) -> bool {
    let Ok(manager) = context.cast::<ITfCompartmentMgr>() else {
        return false;
    };
    flag(&manager, &GUID_COMPARTMENT_KEYBOARD_DISABLED)
        || flag(&manager, &GUID_COMPARTMENT_EMPTYCONTEXT)
}

/// 上下文 compartment 里的 `DWORD` 非零。没设过 / 类型不对按 0。
fn flag(manager: &ITfCompartmentMgr, guid: &GUID) -> bool {
    unsafe { manager.GetCompartment(guid) }
        .and_then(|compartment| unsafe { compartment.GetValue() })
        .ok()
        .and_then(|variant| i32::try_from(&variant).ok())
        .is_some_and(|value| value != 0)
}
