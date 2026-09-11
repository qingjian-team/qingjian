use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{E_FAIL, ERROR_CLASS_ALREADY_EXISTS, GetLastError};
use windows::Win32::UI::WindowsAndMessaging::{RegisterClassExW, WNDCLASSEXW};
use windows::core::{Error, Result};

/// 进程内只注册一次的窗口类，放 `static` 里，每次建窗口前 [`Self::ensure`] 一下。
pub(crate) struct WindowClass {
    once: Once,

    /// 首次注册失败则一直失败。
    failed: AtomicBool,
}

impl WindowClass {
    pub(crate) const fn new() -> Self {
        Self {
            once: Once::new(),
            failed: AtomicBool::new(false),
        }
    }

    /// 首次按 `build` 注册（`cbSize` 由这里填）。类已存在不算失败（DLL 卸载后重载、同进程多个 TIP 实例会撞上）。
    pub(crate) fn ensure(&self, build: impl FnOnce() -> WNDCLASSEXW) -> Result<()> {
        self.once.call_once(|| {
            let mut class = build();
            class.cbSize = size_of::<WNDCLASSEXW>() as u32;
            let atom = unsafe { RegisterClassExW(&class) };
            if atom == 0 && unsafe { GetLastError() } != ERROR_CLASS_ALREADY_EXISTS {
                self.failed.store(true, Ordering::Relaxed);
            }
        });
        if self.failed.load(Ordering::Relaxed) {
            Err(Error::from(E_FAIL))
        } else {
            Ok(())
        }
    }
}
