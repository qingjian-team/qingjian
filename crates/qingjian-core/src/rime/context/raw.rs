//! librime C 布局；指针只在原生输出释放前复制。
use super::composition::Composition;
use super::menu::Menu;
use std::ffi::{c_char, c_int};

#[repr(C)]
#[derive(Default)]
pub(crate) struct Context {
    pub data_size: c_int,

    pub composition: Composition,

    pub menu: Menu,

    pub commit_text_preview: *mut c_char,

    pub select_labels: *mut *mut c_char,
}
