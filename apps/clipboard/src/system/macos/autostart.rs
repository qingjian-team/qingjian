//! 单独安装时的开机启动：登记成系统的登录项（`SMAppService`，macOS 13 起），用户在「系统设置 → 通用 → 登录项」里也看得到、关得掉。
//! 由输入法拉起的那份归输入法管，菜单里不显示这一项。

use std::ptr;

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};

#[link(name = "ServiceManagement", kind = "framework")]
unsafe extern "C" {}

/// `SMAppServiceStatusEnabled`。
const STATUS_ENABLED: isize = 1;

/// 本程序自己这个登录项。
fn main_app() -> Option<Retained<AnyObject>> {
    let class = AnyClass::get(c"SMAppService")?;
    // SAFETY: `+[SMAppService mainAppService]` 没有参数，返回自动释放的对象。
    unsafe { msg_send![class, mainAppService] }
}

pub fn enabled() -> bool {
    main_app().is_some_and(|service| {
        // SAFETY: `status` 返回 NSInteger。
        let status: isize = unsafe { msg_send![&*service, status] };
        status == STATUS_ENABLED
    })
}

pub fn set(on: bool) {
    let Some(service) = main_app() else {
        tracing::warn!("login items are not available");
        return;
    };
    let error: *mut *mut AnyObject = ptr::null_mut();
    // SAFETY: 两个方法都只有一个 NSError** 出参，传空表示不要错误对象；返回 BOOL。
    let done: bool = unsafe {
        if on {
            msg_send![&*service, registerAndReturnError: error]
        } else {
            msg_send![&*service, unregisterAndReturnError: error]
        }
    };
    if !done {
        tracing::warn!(on, "cannot change the login item");
    }
}
