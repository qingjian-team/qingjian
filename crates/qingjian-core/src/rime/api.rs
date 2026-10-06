//! 从带 data_size 的稳定 C API 表取函数。索引来自 librime 1.17.0 rime_api.h。
//! 只读取已检查范围的槽位，不把旧版短表强转为新版完整结构体。
use std::ffi::{c_char, c_int, c_void};
use std::mem::{align_of, size_of};

use super::RimeError;
use super::commit::Commit;
use super::context::Context;
use super::library::Library;
use super::traits::Traits;

pub(super) struct Api {
    pub setup: unsafe extern "C" fn(*mut Traits),

    pub notify: unsafe extern "C" fn(
        Option<unsafe extern "C" fn(*mut c_void, usize, *const c_char, *const c_char)>,
        *mut c_void,
    ),

    pub initialize: unsafe extern "C" fn(*mut Traits),

    pub finalize: unsafe extern "C" fn(),

    pub maintenance: unsafe extern "C" fn(c_int) -> c_int,

    pub join: unsafe extern "C" fn(),

    pub create: unsafe extern "C" fn() -> usize,

    pub destroy: unsafe extern "C" fn(usize) -> c_int,

    pub process: unsafe extern "C" fn(usize, c_int, c_int) -> c_int,

    pub commit: unsafe extern "C" fn(usize) -> c_int,

    pub clear: unsafe extern "C" fn(usize),

    pub get_commit: unsafe extern "C" fn(usize, *mut Commit) -> c_int,

    pub free_commit: unsafe extern "C" fn(*mut Commit) -> c_int,

    pub context: unsafe extern "C" fn(usize, *mut Context) -> c_int,

    pub free_context: unsafe extern "C" fn(*mut Context) -> c_int,

    pub set_option: unsafe extern "C" fn(usize, *const c_char, c_int),

    pub get_option: unsafe extern "C" fn(usize, *const c_char) -> c_int,

    pub current_schema: unsafe extern "C" fn(usize, *mut c_char, usize) -> c_int,

    pub select_schema: unsafe extern "C" fn(usize, *const c_char) -> c_int,

    pub find_module: unsafe extern "C" fn(*const c_char) -> *mut c_void,

    pub input: unsafe extern "C" fn(usize) -> *const c_char,

    pub caret: unsafe extern "C" fn(usize) -> usize,

    pub select: unsafe extern "C" fn(usize, usize) -> c_int,

    pub set_caret: unsafe extern "C" fn(usize, usize),

    pub delete: unsafe extern "C" fn(usize, usize) -> c_int,

    pub set_input: unsafe extern "C" fn(usize, *const c_char) -> c_int,

    pub change_page: unsafe extern "C" fn(usize, c_int) -> c_int,
}

impl Api {
    /// 安全约束：用户选择的原生库必须遵守 rime_get_api 的 ABI。
    pub unsafe fn load(library: &Library) -> Result<Self, RimeError> {
        let getter = unsafe { library.api_getter()? };
        let table = unsafe { getter() };
        if table.is_null() {
            return Err(RimeError::Api("rime_get_api"));
        }
        let length = unsafe { table.read() }.max(0) as usize + size_of::<c_int>();
        // C 在第一个指针前插入对齐填充；data_size 只扣除 int 的大小。
        let start = size_of::<c_int>().next_multiple_of(align_of::<*const c_void>());
        macro_rules! slot {
            ($index:literal, $name:literal, $signature:ty) => {{
                let offset = start + $index * size_of::<*const c_void>();
                if offset + size_of::<*const c_void>() > length {
                    return Err(RimeError::Api($name));
                }
                let pointer = unsafe {
                    table
                        .cast::<u8>()
                        .add(offset)
                        .cast::<*const c_void>()
                        .read()
                };
                if pointer.is_null() {
                    return Err(RimeError::Api($name));
                }
                // 目标函数类型必须与 rime_api.h 中的声明完全一致。
                unsafe { std::mem::transmute::<*const c_void, $signature>(pointer) }
            }};
        }
        Ok(Self {
            setup: slot!(0, "setup", unsafe extern "C" fn(*mut Traits)),
            initialize: slot!(2, "initialize", unsafe extern "C" fn(*mut Traits)),
            finalize: slot!(3, "finalize", unsafe extern "C" fn()),
            notify: slot!(
                1,
                "set_notification_handler",
                unsafe extern "C" fn(
                    Option<unsafe extern "C" fn(*mut c_void, usize, *const c_char, *const c_char)>,
                    *mut c_void,
                )
            ),
            maintenance: slot!(4, "start_maintenance", unsafe extern "C" fn(c_int) -> c_int),
            join: slot!(6, "join_maintenance_thread", unsafe extern "C" fn()),
            create: slot!(13, "create_session", unsafe extern "C" fn() -> usize),
            destroy: slot!(15, "destroy_session", unsafe extern "C" fn(usize) -> c_int),
            process: slot!(
                18,
                "process_key",
                unsafe extern "C" fn(usize, c_int, c_int) -> c_int
            ),
            commit: slot!(
                19,
                "commit_composition",
                unsafe extern "C" fn(usize) -> c_int
            ),
            clear: slot!(20, "clear_composition", unsafe extern "C" fn(usize)),
            get_commit: slot!(
                21,
                "get_commit",
                unsafe extern "C" fn(usize, *mut Commit) -> c_int
            ),
            free_commit: slot!(
                22,
                "free_commit",
                unsafe extern "C" fn(*mut Commit) -> c_int
            ),
            context: slot!(
                23,
                "get_context",
                unsafe extern "C" fn(usize, *mut Context) -> c_int
            ),
            free_context: slot!(
                24,
                "free_context",
                unsafe extern "C" fn(*mut Context) -> c_int
            ),
            set_option: slot!(
                27,
                "set_option",
                unsafe extern "C" fn(usize, *const c_char, c_int)
            ),
            select_schema: slot!(
                34,
                "select_schema",
                unsafe extern "C" fn(usize, *const c_char) -> c_int
            ),
            current_schema: slot!(
                33,
                "get_current_schema",
                unsafe extern "C" fn(usize, *mut c_char, usize) -> c_int
            ),
            find_module: slot!(
                49,
                "find_module",
                unsafe extern "C" fn(*const c_char) -> *mut c_void
            ),
            get_option: slot!(
                28,
                "get_option",
                unsafe extern "C" fn(usize, *const c_char) -> c_int
            ),
            input: slot!(
                69,
                "get_input",
                unsafe extern "C" fn(usize) -> *const c_char
            ),
            caret: slot!(70, "get_caret_pos", unsafe extern "C" fn(usize) -> usize),
            select: slot!(
                71,
                "select_candidate",
                unsafe extern "C" fn(usize, usize) -> c_int
            ),
            set_caret: slot!(73, "set_caret_pos", unsafe extern "C" fn(usize, usize)),
            delete: slot!(
                86,
                "delete_candidate",
                unsafe extern "C" fn(usize, usize) -> c_int
            ),
            set_input: slot!(
                89,
                "set_input",
                unsafe extern "C" fn(usize, *const c_char) -> c_int
            ),
            change_page: slot!(
                97,
                "change_page",
                unsafe extern "C" fn(usize, c_int) -> c_int
            ),
        })
    }
}
