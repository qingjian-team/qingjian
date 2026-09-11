//! `cfg(windows)`：TSF 文本服务的 COM 外壳。
//!
//! 导出 COM 约定的 DLL 入口：[`DllGetClassObject`]、[`DllCanUnloadNow`]、[`DllRegisterServer`] /
//! [`DllUnregisterServer`]，外加 [`DllMain`] 记下模块句柄。类厂在 [`factory`]，文本服务对象在 [`service`]，
//! 注册表 / TSF profile 在 [`registry`]。
#![allow(non_snake_case)] // 导出的 Dll* 入口按 COM 约定命名

pub(crate) mod composition;
pub(crate) mod display_attribute;
pub(crate) mod edit_session;
pub(crate) mod factory;
pub(crate) mod hook;
pub(crate) mod icon;
pub(crate) mod keys;
pub(crate) mod langbar;
pub(crate) mod log;
pub(crate) mod mode;
pub(crate) mod poll;
pub(crate) mod registry;
pub(crate) mod service;
mod variant;
pub(crate) mod window_class;

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicIsize, AtomicPtr, Ordering};

use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, E_FAIL, HINSTANCE, HMODULE, S_FALSE, S_OK,
};
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::core::{BOOL, GUID, HRESULT, HSTRING, Interface};

/// 文本服务的 CLSID。注册表 InprocServer32、TSF profile、[`DllGetClassObject`] 都认它。
pub(crate) const CLSID_QINGJIAN: GUID = GUID::from_u128(0x4fdca82d_e923_49bf_9e75_bb906b93b8bb);

/// [`CLSID_QINGJIAN`] 的注册表字符串形式，改一个必须同步改另一个。
pub(crate) const CLSID_QINGJIAN_STR: &str = "{4FDCA82D-E923-49BF-9E75-BB906B93B8BB}";

/// 语言 profile 的 GUID。
pub(crate) const GUID_PROFILE: GUID = GUID::from_u128(0x8119f8e0_cf81_423b_9189_c0d7374324b3);

/// zh-CN。
pub(crate) const LANGID_ZH_CN: u16 = 0x0804;

/// 输入法在系统里显示的名字。
pub(crate) const SERVICE_DESCRIPTION: &str = "青简";

/// 存活的 COM 对象 + LockServer 计数，[`DllCanUnloadNow`] 据它判断能否卸载。
static DLL_REFERENCES: AtomicIsize = AtomicIsize::new(0);

/// 本 DLL 的模块句柄，[`DllMain`] 加载时记下。
static DLL_MODULE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

pub(crate) fn lock_module() {
    DLL_REFERENCES.fetch_add(1, Ordering::SeqCst);
}

pub(crate) fn unlock_module() {
    DLL_REFERENCES.fetch_sub(1, Ordering::SeqCst);
}

/// 本 DLL 的实例句柄（注册窗口类 / 建窗口用）。未记时为 null，系统回落到进程 exe。
pub(crate) fn dll_instance() -> HINSTANCE {
    HINSTANCE(DLL_MODULE.load(Ordering::SeqCst))
}

/// 本 DLL 在磁盘上的完整路径，注册 InprocServer32 用。
pub(crate) fn module_path() -> windows::core::Result<HSTRING> {
    let module = HMODULE(DLL_MODULE.load(Ordering::SeqCst));
    let path = file_name_of(Some(module))?;
    Ok(HSTRING::from_wide(&path))
}

/// 宿主应用的 exe 文件名（`Code.exe`）：DLL 加载在应用进程里，取当前进程 exe 的路径去掉目录即可。
/// 开会话时报给 Server，对应 macOS 端的 bundle identifier；取不到为 `None`。
pub(crate) fn host_app_name() -> Option<String> {
    let path = file_name_of(None).ok()?;
    let start = path
        .iter()
        .rposition(|&unit| unit == u16::from(b'\\') || unit == u16::from(b'/'))
        .map_or(0, |slash| slash + 1);
    let name = String::from_utf16_lossy(&path[start..]);
    (!name.is_empty()).then_some(name)
}

/// `module` 的完整路径（UTF-16，不含结尾 0）；`None` 是当前进程的 exe。
fn file_name_of(module: Option<HMODULE>) -> windows::core::Result<Vec<u16>> {
    let mut buf = [0u16; 260];
    // SAFETY: buf 可写；module 是本 DLL 的句柄或 None（取当前进程 exe）。
    let len = unsafe { GetModuleFileNameW(module, &mut buf) } as usize;
    if len == 0 || len >= buf.len() {
        return Err(E_FAIL.into());
    }
    Ok(buf[..len].to_vec())
}

#[unsafe(no_mangle)]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_FAIL;
    }
    // SAFETY: 已判非空。
    if unsafe { *rclsid } != CLSID_QINGJIAN {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = factory::ClassFactory.into();
    // SAFETY: riid 指向有效 GUID，ppv 可写一个接口指针。
    unsafe { factory.query(riid, ppv) }
}

#[unsafe(no_mangle)]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    if DLL_REFERENCES.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllRegisterServer() -> HRESULT {
    match registry::register() {
        Ok(()) => S_OK,
        Err(error) => {
            log::log(&format!("DllRegisterServer 失败: {error}"));
            error.code()
        }
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllUnregisterServer() -> HRESULT {
    match registry::unregister() {
        Ok(()) => S_OK,
        Err(error) => error.code(),
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllMain(hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DLL_MODULE.store(hinst.0, Ordering::SeqCst);
        log::log("DllMain: DLL_PROCESS_ATTACH");
    }
    true.into()
}
