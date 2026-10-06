//! RimeTraits C 布局（rime_api.h，BSD-3-Clause，Rime Developers）。
use std::ffi::{c_char, c_int};

#[repr(C)]
pub(super) struct Traits {
    pub data_size: c_int,

    pub shared_data_dir: *const c_char,

    pub user_data_dir: *const c_char,

    pub distribution_name: *const c_char,

    pub distribution_code_name: *const c_char,

    pub distribution_version: *const c_char,

    pub app_name: *const c_char,

    pub modules: *const *const c_char,

    pub min_log_level: c_int,

    pub log_dir: *const c_char,

    pub prebuilt_data_dir: *const c_char,

    pub staging_dir: *const c_char,
}
