//! 候选修正输入框：用系统原生警告框收一段正确文本。

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSApplication, NSApplicationActivationPolicy, NSTextField,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

pub(super) fn edit_translation(current: &str, language: &str) -> Option<String> {
    let mtm = MainThreadMarker::new()?;
    tracing::info!(%language, "打开候选译文输入框");
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str("修改候选译文"));
    alert.setInformativeText(&NSString::from_str(&format!(
        "输入正确的{language}译文；保存后会优先于内置释义显示。"
    )));
    alert.addButtonWithTitle(&NSString::from_str("保存译文"));
    alert.addButtonWithTitle(&NSString::from_str("取消"));
    let field = NSTextField::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::ZERO, NSSize::new(320.0, 24.0)),
    );
    field.setStringValue(&NSString::from_str(current));
    alert.setAccessoryView(Some(&field));
    let window = alert.window();
    window.setInitialFirstResponder(Some(&field));
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    window.makeKeyAndOrderFront(None);
    if alert.runModal() != NSAlertFirstButtonReturn {
        return None;
    }
    let text = field.stringValue().to_string();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}
