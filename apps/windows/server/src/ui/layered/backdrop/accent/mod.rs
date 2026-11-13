//! `SetWindowCompositionAttribute` 的窗口强调效果（未公开 API，Windows 10 1803 起有亚克力）：user32 里按名字取，取不到就没有材质。

mod data;
mod policy;

use std::sync::OnceLock;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::core::{BOOL, s, w};

use self::data::CompositionData;
use self::policy::AccentPolicy;

/// `WCA_ACCENT_POLICY`。
const WCA_ACCENT_POLICY: u32 = 19;

/// `ACCENT_DISABLED`：撤掉效果。
pub(super) const ACCENT_DISABLED: u32 = 0;

/// `ACCENT_ENABLE_ACRYLICBLURBEHIND`：亚克力（模糊 + 着色 + 噪点）。
pub(super) const ACCENT_ACRYLIC: u32 = 4;

type SetWindowCompositionAttribute = unsafe extern "system" fn(HWND, *mut CompositionData) -> BOOL;

fn entry() -> Option<SetWindowCompositionAttribute> {
    static ENTRY: OnceLock<Option<SetWindowCompositionAttribute>> = OnceLock::new();
    *ENTRY.get_or_init(|| {
        // SAFETY: user32 随进程常驻；签名照系统导出的原型
        unsafe {
            let user32 = GetModuleHandleW(w!("user32.dll")).ok()?;
            let address = GetProcAddress(user32, s!("SetWindowCompositionAttribute"))?;
            Some(std::mem::transmute::<
                unsafe extern "system" fn() -> isize,
                SetWindowCompositionAttribute,
            >(address))
        }
    })
}

/// 系统有这个 API。
pub(super) fn available() -> bool {
    entry().is_some()
}

/// 设窗口的强调效果；`color` 是 `0xAABBGGRR` 着色。成功返回真。
pub(super) fn set(hwnd: HWND, state: u32, color: u32) -> bool {
    let Some(set) = entry() else {
        return false;
    };
    let mut policy = AccentPolicy {
        state,
        flags: 0,
        gradient_color: color,
        animation_id: 0,
    };
    let mut data = CompositionData {
        attribute: WCA_ACCENT_POLICY,
        data: (&raw mut policy).cast(),
        size: size_of::<AccentPolicy>(),
    };
    // SAFETY: data 指向栈上的 policy，调用期间有效
    unsafe { set(hwnd, &mut data) }.as_bool()
}
