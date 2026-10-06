//! librime C 布局；指针只在原生输出释放前复制。
use super::candidate::Candidate;
use std::ffi::{c_char, c_int};

#[repr(C)]
#[derive(Default)]
pub(crate) struct Menu {
    pub page_size: c_int,

    pub page_no: c_int,

    pub is_last_page: c_int,

    pub highlighted_candidate_index: c_int,

    pub num_candidates: c_int,

    pub candidates: *mut Candidate,

    pub select_keys: *mut c_char,
}
