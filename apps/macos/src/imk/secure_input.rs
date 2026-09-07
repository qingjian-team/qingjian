//! Secure Input（密码框等）检测。开着的时候光标附近的文本绝不发往云端。

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}

/// 系统当前是否处于安全输入状态。
pub fn enabled() -> bool {
    // SAFETY: 无参数的纯查询函数，Carbon 框架在 macOS 上始终可用。
    unsafe { IsSecureEventInputEnabled() }
}
