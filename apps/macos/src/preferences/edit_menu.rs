use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

/// 给进程装一份只有「编辑」的主菜单。
///
/// 输入法是 `LSBackgroundOnly`，从不显示菜单栏，也就没有主菜单；而 AppKit 文本框里的 ⌘X / ⌘C / ⌘V / ⌘A / ⌘Z
/// 是主菜单里对应条目的快捷键，菜单不存在这些键就没人响应——偏好设置里的密钥框粘不进去就是这个原因。
/// 菜单栏本身永远不会画出来（激活策略最多到 Accessory），只是让快捷键有地方落。重复调用只装一次。
pub fn install(mtm: MainThreadMarker) {
    let app = NSApplication::sharedApplication(mtm);
    if app.mainMenu().is_some() {
        return;
    }
    let edit = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("编辑"));
    for (title, action, key) in [
        ("撤销", sel!(undo:), "z"),
        ("重做", sel!(redo:), "Z"),
        ("剪切", sel!(cut:), "x"),
        ("拷贝", sel!(copy:), "c"),
        ("粘贴", sel!(paste:), "v"),
        ("全选", sel!(selectAll:), "a"),
    ] {
        // SAFETY: 这些都是 NSResponder 的标准编辑选择器，target 为空时沿响应链送到当前编辑的文本框
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str(title),
                Some(action),
                &NSString::from_str(key),
            )
        };
        edit.addItem(&item);
    }
    let main: Retained<NSMenu> = NSMenu::new(mtm);
    let holder = NSMenuItem::new(mtm);
    holder.setSubmenu(Some(&edit));
    main.addItem(&holder);
    app.setMainMenu(Some(&main));
}
