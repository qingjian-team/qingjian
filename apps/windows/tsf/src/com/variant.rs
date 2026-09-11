//! 手搓 `VT_I4` 的 VARIANT：windows 0.62 没给 `VARIANT::From<i32>`，好几处（转换模式 compartment、
//! 组句显示属性 atom）都要往 TSF 传一个整数值，统一在这里造。

use core::mem::ManuallyDrop;

use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4};

/// 造一个 `VT_I4` 的 VARIANT。
pub(super) fn i4(value: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_I4,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { lVal: value },
            }),
        },
    }
}
