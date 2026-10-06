//! librime 是进程级服务。兼容配置共享一个实例，冲突配置在初始化前拒绝。
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};

use super::api::Api;
use super::library::Library;
use super::traits::Traits;
use super::{RimeError, RimeOptions};

// bool 在 setup 成功后置位，finalize 完成后清除。最后一个 Arc 正在析构时，
// 新调用等待 finalize，不能在 Weak::upgrade 失败后直接覆盖仍运行的 C 全局服务。
static RUNTIME: Mutex<(Weak<Runtime>, bool)> = Mutex::new((Weak::new(), false));
static FINALIZED: Condvar = Condvar::new();

pub(super) struct Runtime {
    pub api: Api,

    pub options: RimeOptions,

    calls: Mutex<()>,

    // C 保存的指针与 DLL 必须活到 finalize 之后。
    _strings: Vec<CString>,

    _deployment_failed: Box<AtomicBool>,

    _library: Library,
}

impl Runtime {
    pub fn open(mut options: RimeOptions) -> Result<Arc<Self>, RimeError> {
        options.library = options.library.canonicalize()?;
        options.shared_data = options.shared_data.canonicalize()?;
        if !options.shared_data.is_dir() {
            return Err(RimeError::InvalidPath);
        }
        std::fs::create_dir_all(&options.user_data)?;
        options.user_data = options.user_data.canonicalize()?;
        let mut registry = RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(runtime) = registry.0.upgrade() {
                let same = runtime.options.library == options.library
                    && runtime.options.shared_data == options.shared_data
                    && runtime.options.user_data == options.user_data
                    && runtime.options.modules == options.modules;
                drop(registry);
                return if same {
                    Ok(runtime)
                } else {
                    Err(RimeError::RuntimeConflict)
                };
            }
            if !registry.1 {
                break;
            }
            registry = FINALIZED.wait(registry).unwrap_or_else(|e| e.into_inner());
        }
        let mut strings = vec![
            path_string(&options.shared_data)?,
            path_string(&options.user_data)?,
            CString::new(env!("CARGO_PKG_VERSION")).expect("package version has no NUL"),
        ];
        // 安全约束：用户显式选择可执行的原生库；加载时检查所需 API 槽位及 C 签名。
        let library = unsafe { Library::new(&options.library)? };
        let api = unsafe { Api::load(&library)? };
        let mut modules = vec![c"default".as_ptr(), c"lua".as_ptr()];
        for name in &options.modules {
            strings.push(CString::new(name.as_str()).map_err(|_| RimeError::InvalidPath)?);
            modules.push(strings.last().expect("module name was appended").as_ptr());
        }
        modules.push(std::ptr::null());
        let mut traits = Traits {
            data_size: (std::mem::size_of::<Traits>() - std::mem::size_of::<i32>()) as i32,
            shared_data_dir: strings[0].as_ptr(),
            user_data_dir: strings[1].as_ptr(),
            distribution_name: c"Qingjian".as_ptr(),
            distribution_code_name: c"qingjian".as_ptr(),
            distribution_version: strings[2].as_ptr(),
            app_name: c"rime.qingjian".as_ptr(),
            modules: modules.as_ptr(),
            min_log_level: 2,
            log_dir: strings[1].as_ptr(),
            prebuilt_data_dir: std::ptr::null(),
            staging_dir: std::ptr::null(),
        };
        let failed = Box::new(AtomicBool::new(false));
        unsafe {
            (api.setup)(&mut traits);
            (api.initialize)(&mut traits);
            if (api.find_module)(c"lua".as_ptr()).is_null() {
                (api.finalize)();
                return Err(RimeError::LuaMissing);
            }
            for name in &options.modules {
                let module = CString::new(name.as_str()).map_err(|_| RimeError::InvalidPath)?;
                if (api.find_module)(module.as_ptr()).is_null() {
                    (api.finalize)();
                    return Err(RimeError::ModuleMissing(name.clone()));
                }
            }
            (api.notify)(
                Some(deployment_notification),
                (&*failed as *const AtomicBool).cast_mut().cast(),
            );
            (api.maintenance)(1);
            (api.join)();
            if failed.load(Ordering::Acquire) {
                (api.notify)(None, std::ptr::null_mut());
                (api.finalize)();
                return Err(RimeError::Deployment);
            }
        }
        let runtime = Arc::new(Self {
            api,
            options,
            calls: Mutex::new(()),
            _strings: strings,
            _deployment_failed: failed,
            _library: library,
        });
        *registry = (Arc::downgrade(&runtime), true);
        Ok(runtime)
    }

    pub fn lock(&self) -> MutexGuard<'_, ()> {
        self.calls.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // 与新实例的 setup 串行化；最后一个会话先 destroy，再执行 finalize。
        let mut registry = RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            (self.api.notify)(None, std::ptr::null_mut());
            (self.api.finalize)();
        }
        *registry = (Weak::new(), false);
        FINALIZED.notify_all();
    }
}

fn path_string(path: &Path) -> Result<CString, RimeError> {
    let path = path.to_str().ok_or(RimeError::InvalidPath)?;
    // Windows canonicalize 的扩展前缀不被 librime 的 std::filesystem 接受。
    let path = if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    };
    CString::new(path).map_err(|_| RimeError::InvalidPath)
}

unsafe extern "C" fn deployment_notification(
    context: *mut c_void,

    _session: usize,

    kind: *const c_char,

    value: *const c_char,
) {
    if !context.is_null()
        && !kind.is_null()
        && !value.is_null()
        && unsafe { CStr::from_ptr(kind) }.to_bytes() == b"deploy"
        && unsafe { CStr::from_ptr(value) }.to_bytes() == b"failure"
    {
        unsafe { &*context.cast::<AtomicBool>() }.store(true, Ordering::Release);
    }
}
