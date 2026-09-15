//! 偏好设置窗口：配置文件的图形前端。
//!
//! 普通控件改完立即写回 `config.toml`（`Config::set_value`，保留注释），
//! 再经 [`crate::host::Host::apply_config`] 生效；窗口自己不存任何状态，勾选与文本永远从配置同步过来。
//! 自定义短语通过保存按钮校验后写入；密钥例外：写到配置同目录的 `.env`，不进 `config.toml`。
//!
//! 输入法进程是 `LSBackgroundOnly`，平时不能成为前台应用；打开窗口前把激活策略临时切成 Accessory，
//! 关窗时切回去，否则文本框拿不到键盘焦点。文本框里的 ⌘C / ⌘V 靠主菜单「编辑」项的快捷键分发，
//! 后台应用没有主菜单，所以开窗前装一份只有编辑项的主菜单（`edit_menu`）。

mod controls;
mod edit_menu;
mod file_dialog;
mod key_recorder;
mod layout;
mod pager;
mod pages;
mod panel;
mod setting;
mod sidebar;
mod target;
mod window;

use objc2::runtime::AnyObject;
use objc2_app_kit::{NSButton, NSControlStateValueOn, NSPopUpButton, NSTextField};

pub use file_dialog::choose_dictionary_file;
pub use key_recorder::KeyRecorder;
pub use pages::{REPOSITORY_URL, WEBSITE_URL};
pub use setting::{Setting, SettingValue};
pub use window::PreferencesWindow;

/// 从 `changed:` 的 sender 认出是哪个设置、现在的值是什么。
pub fn setting_from_sender(sender: Option<&AnyObject>) -> Option<(Setting, SettingValue)> {
    let sender = sender?;
    // 快捷键录制按钮是 NSButton 的子类，最先认它：值是它录到的配置写法
    if let Some(recorder) = sender.downcast_ref::<KeyRecorder>() {
        return Some((
            Setting::from_tag(recorder.tag())?,
            SettingValue::Text(recorder.recorded()),
        ));
    }
    // NSPopUpButton 是 NSButton 的子类，先认它
    if let Some(popup) = sender.downcast_ref::<NSPopUpButton>() {
        let index = usize::try_from(popup.indexOfSelectedItem()).ok()?;
        return Some((Setting::from_tag(popup.tag())?, SettingValue::Index(index)));
    }
    if let Some(button) = sender.downcast_ref::<NSButton>() {
        let on = button.state() == NSControlStateValueOn;
        return Some((Setting::from_tag(button.tag())?, SettingValue::Bool(on)));
    }
    if let Some(field) = sender.downcast_ref::<NSTextField>() {
        let text = field.stringValue().to_string();
        return Some((Setting::from_tag(field.tag())?, SettingValue::Text(text)));
    }
    None
}

/// 开发用预览入口（`qingjian-macos --preferences-preview [页序号] [dark]`）：用缺省配置、空统计与两种释义表语言
/// 把设置窗口直接打开在前台，不依赖 IMK 与 Host；控件改动因为没有 Host 会被静默忽略。截图与调布局用。
pub fn preview_main() {
    use objc2_app_kit::{NSAppearance, NSAppearanceNameDarkAqua, NSApplication};
    use qingjian_core::{Language, UsageSummary, VocabularySummary};
    use qingjian_platform::Config;

    let mtm = objc2::MainThreadMarker::new().expect("预览入口必须在主线程");
    let app = NSApplication::sharedApplication(mtm);
    let extra: Vec<String> = std::env::args()
        .skip_while(|a| a != "--preferences-preview")
        .skip(1)
        .collect();
    let page = extra
        .first()
        .and_then(|a| a.parse::<usize>().ok())
        .unwrap_or(0);
    if extra.iter().any(|a| a == "dark") {
        // SAFETY: 外观名是 AppKit 导出的常量
        let dark = unsafe { NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua) };
        app.setAppearance(dark.as_deref());
    }
    let languages = [Language::English, Language::Japanese];
    let window = PreferencesWindow::new(mtm, &languages, "预览", "dev · 本地");
    window.sync(&Config::default(), false, None, &[]);
    window.sync_usage(
        &UsageSummary::default(),
        &VocabularySummary::default(),
        Language::English,
    );
    window.select_page(page);
    window.show();
    app.run();
}
