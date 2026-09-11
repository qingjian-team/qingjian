//! 反向同步：监听「转换模式」compartment 的变化（用户点任务栏中 / 英），回写 DLL 的英文模式。
//! 写在 [`super`]（`mode`）；自己写的那次触发的 `OnChange` 读回与当前相同，在 service 里比较相等即忽略。

use windows::Win32::UI::TextServices::{
    GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION, ITfCompartmentEventSink,
    ITfCompartmentEventSink_Impl, ITfSource, ITfThreadMgr,
};
use windows::core::{GUID, Interface, Result, implement};

#[implement(ITfCompartmentEventSink)]
pub(crate) struct ConversionSink;

impl ITfCompartmentEventSink_Impl for ConversionSink_Impl {
    fn OnChange(&self, rguid: *const GUID) -> Result<()> {
        if unsafe { *rguid } == GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION {
            crate::com::service::on_conversion_mode_changed();
        }
        Ok(())
    }
}

/// 挂上事件回调，返回 `(source, cookie)` 供 [`unadvise`]。
pub(crate) fn advise(thread_mgr: &ITfThreadMgr) -> Result<(ITfSource, u32)> {
    let source: ITfSource = super::conversion_compartment(thread_mgr)?.cast()?;
    let sink: ITfCompartmentEventSink = ConversionSink.into();
    let cookie = unsafe { source.AdviseSink(&ITfCompartmentEventSink::IID, &sink)? };
    Ok((source, cookie))
}

pub(crate) fn unadvise(source: &ITfSource, cookie: u32) {
    let _ = unsafe { source.UnadviseSink(cookie) };
}
