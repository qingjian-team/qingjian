//! librime C 布局；指针只在原生输出释放前复制。
use std::ffi::{c_char, c_void};

#[repr(C)]
pub(crate) struct Candidate {
    pub text: *mut c_char,

    pub comment: *mut c_char,

    pub reserved: *mut c_void,
}
