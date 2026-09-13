//! 偏好设置窗口：配置文件的图形前端。
//!
//! 普通控件改完立即写回 `config.toml`（`Config::set_value`，保留注释），
//! 再经 [`crate::host::Host::apply_config`] 生效；窗口自己不存任何状态，勾选与文本永远从配置同步过来。
//! 自定义文本通过保存按钮校验后写入；密钥例外：写到配置同目录的 `.env`，不进 `config.toml`。
//!
//! 输入法进程是 `LSBackgroundOnly`，平时不能成为前台应用；打开窗口前把激活策略临时切成 Accessory，
//! 关窗时切回去，否则文本框拿不到键盘焦点。文本框里的 ⌘C / ⌘V 靠主菜单「编辑」项的快捷键分发，
//! 后台应用没有主菜单，所以开窗前装一份只有编辑项的主菜单（`edit_menu`）。

mod controls;
mod edit_menu;
mod file_dialog;
mod key_recorder;
mod layout;
mod pages;
mod panel;
mod setting;
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
