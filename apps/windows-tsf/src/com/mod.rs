//! `cfg(windows)`：TSF 文本服务的 COM 外壳——被 Windows 加载、注册的那一层。
//!
//! 导出四个 COM 约定的 DLL 入口：[`DllGetClassObject`]（给出造对象的类厂）、[`DllCanUnloadNow`]、
//! [`DllRegisterServer`] / [`DllUnregisterServer`]（regsvr32 用它自注册 / 反注册），外加 [`DllMain`]
//! 记下模块句柄。类厂在 [`factory`]，文本服务对象在 [`service`]，注册表 / TSF profile 落地在 [`registry`]。
//!
//! 这一步只打通最小链路：系统能造出对象、Activate、经 [`ITfKeyEventSink`](windows::Win32::UI::TextServices::ITfKeyEventSink)
//! 收到按键并转发给 Server（引擎在 Server 进程）。把候选画出来 / 经编辑会话上屏是下一步。
#![allow(non_snake_case)] // 导出的 Dll* 入口按 COM 约定用帕斯卡命名

pub(crate) mod factory;
pub(crate) mod registry;
pub(crate) mod service;

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

/// 与 [`CLSID_QINGJIAN`] 对应的注册表字符串（大写带花括号）。改一个必须同步改另一个。
pub(crate) const CLSID_QINGJIAN_STR: &str = "{4FDCA82D-E923-49BF-9E75-BB906B93B8BB}";

/// 语言 profile 的 GUID（一个文本服务下可有多个 profile，我们先一个）。
pub(crate) const GUID_PROFILE: GUID = GUID::from_u128(0x8119f8e0_cf81_423b_9189_c0d7374324b3);

/// 简体中文 langid（zh-CN）。
pub(crate) const LANGID_ZH_CN: u16 = 0x0804;

/// 输入法在系统里显示的名字。
pub(crate) const SERVICE_DESCRIPTION: &str = "青简";

/// 存活的 COM 对象 + LockServer 计数，[`DllCanUnloadNow`] 据它判断能否卸载。
static DLL_REFERENCES: AtomicIsize = AtomicIsize::new(0);

/// 本 DLL 的模块句柄，[`DllMain`] 在加载时记下，注册时用它取自身路径。
static DLL_MODULE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

/// 增一个模块引用（造对象 / LockServer(true)）。
pub(crate) fn lock_module() {
    DLL_REFERENCES.fetch_add(1, Ordering::SeqCst);
}

/// 减一个模块引用（对象析构 / LockServer(false)）。
pub(crate) fn unlock_module() {
    DLL_REFERENCES.fetch_sub(1, Ordering::SeqCst);
}

/// 本 DLL 在磁盘上的完整路径，注册 InprocServer32 用。
pub(crate) fn module_path() -> windows::core::Result<HSTRING> {
    let module = HMODULE(DLL_MODULE.load(Ordering::SeqCst));
    let mut buf = [0u16; 260];
    // SAFETY: buf 可写；module 是本 DLL 加载时记下的句柄（null 则取当前进程 exe，也能兜底）。
    let len = unsafe { GetModuleFileNameW(Some(module), &mut buf) } as usize;
    if len == 0 || len >= buf.len() {
        return Err(E_FAIL.into());
    }
    Ok(HSTRING::from_wide(&buf[..len]))
}

/// COM 运行时用它拿类厂（[`factory::ClassFactory`]）来造文本服务对象。
#[unsafe(no_mangle)]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_FAIL;
    }
    // SAFETY: 上面已判非空。
    if unsafe { *rclsid } != CLSID_QINGJIAN {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = factory::ClassFactory.into();
    // SAFETY: riid 指向有效 GUID，ppv 可写一个接口指针。
    unsafe { factory.query(riid, ppv) }
}

/// 没有存活对象 / 锁时（计数为 0）才允许卸载。
#[unsafe(no_mangle)]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    if DLL_REFERENCES.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

/// regsvr32 调用：写 InprocServer32 并向 TSF 注册文本服务 / 语言 profile / 类别。
#[unsafe(no_mangle)]
extern "system" fn DllRegisterServer() -> HRESULT {
    match registry::register() {
        Ok(()) => S_OK,
        Err(error) => {
            tracing::error!(%error, "DllRegisterServer 失败");
            error.code()
        }
    }
}

/// regsvr32 /u 调用：撤销 [`DllRegisterServer`] 的注册。
#[unsafe(no_mangle)]
extern "system" fn DllUnregisterServer() -> HRESULT {
    match registry::unregister() {
        Ok(()) => S_OK,
        Err(error) => error.code(),
    }
}

/// DLL 加载 / 卸载回调：只在加载时记下模块句柄。
#[unsafe(no_mangle)]
extern "system" fn DllMain(hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DLL_MODULE.store(hinst.0, Ordering::SeqCst);
    }
    true.into()
}
