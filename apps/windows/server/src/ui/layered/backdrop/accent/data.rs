//! `WINDOWCOMPOSITIONATTRIBDATA`：设哪个属性、值在哪、多大。

use std::ffi::c_void;

#[repr(C)]
pub(super) struct CompositionData {
    /// `WCA_*` 之一。
    pub(super) attribute: u32,

    pub(super) data: *mut c_void,

    pub(super) size: usize,
}
