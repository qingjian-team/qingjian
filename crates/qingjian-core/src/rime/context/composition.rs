//! librime C 布局；指针只在原生输出释放前复制。
use std::ffi::{c_char, c_int};

#[repr(C)]
#[derive(Default)]
pub(crate) struct Composition {
    pub length: c_int,

    pub cursor_pos: c_int,

    pub sel_start: c_int,

    pub sel_end: c_int,

    pub preedit: *mut c_char,
}
