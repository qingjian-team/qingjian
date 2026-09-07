//! 菜单栏：「中 / 英」状态项与输入法菜单。
//!
//! 输入法菜单是一份 NSMenu 同时挂在「中 / 英」状态项（点击弹出）和系统输入源菜单（IMK `menu` 回调）上。
//!
//! 菜单只是配置文件的运行时前端：每个开关点下去先改 `config.toml` 再热加载，不另存一套状态；
//! 手改配置文件与点菜单走的是同一条 [`crate::host::Host::apply_config`] 通路。
//!
//! 两处的 action 派发形态不同：状态项菜单的 sender 是 NSMenuItem 本身；输入源菜单由 IMK 转发到
//! 控制器，sender 是带 `IMKCommandMenuItem` 键的字典。[`action_from_sender`] 两种都认，
//! 动作本身编码在菜单项的 tag 里，所以两边只需要一个 `menuAction:` 选择器。

mod action;
mod indicator;
mod menu;
mod target;

use objc2::runtime::AnyObject;
use objc2_app_kit::NSMenuItem;
use objc2_foundation::NSDictionary;
use objc2_input_method_kit::kIMKCommandMenuItemName;

pub use action::MenuAction;
pub use indicator::ModeIndicator;
pub use menu::InputMenu;

/// 从 `menuAction:` 的 sender 里认出被点的是哪一项。
pub fn action_from_sender(sender: Option<&AnyObject>) -> Option<MenuAction> {
    let sender = sender?;
    let tag = if let Some(item) = sender.downcast_ref::<NSMenuItem>() {
        item.tag()
    } else {
        let info = sender.downcast_ref::<NSDictionary<AnyObject, AnyObject>>()?;
        // SAFETY: 只读 IMK 导出的常量字符串
        let key: &AnyObject = unsafe { kIMKCommandMenuItemName };
        info.objectForKey(key)?.downcast::<NSMenuItem>().ok()?.tag()
    };
    MenuAction::from_tag(tag)
}
