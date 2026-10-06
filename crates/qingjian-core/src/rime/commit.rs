//! librime C 布局；指针只在原生输出释放前复制。
use std::ffi::{c_char, c_int};

#[repr(C)]
#[derive(Default)]
pub(super) struct Commit {
    pub data_size: c_int,

    pub text: *mut c_char,
}
