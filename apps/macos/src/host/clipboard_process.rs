//! 按 `[clipboard] enabled` 拉起青简剪贴板（实验功能，`Contents/Applications/QingjianClipboard.app`）。
//! 关掉时不用管：它自己看配置退出，只开一个也由它自己保证。用 `open` 拉起而不是直接跑二进制，
//! 系统才把它当独立的程序：辅助功能权限记在剪贴板名下，不算到输入法头上。

use objc2_app_kit::NSRunningApplication;
use objc2_foundation::NSString;

use super::diagnostics::open_with_system;

const BUNDLE_ID: &str = "app.qingjian.clipboard";

/// 随包的剪贴板；直接跑二进制（不在 .app 里）或包里缺它时为 `None`。
fn clipboard_app() -> Option<std::path::PathBuf> {
    let bundle = objc2_foundation::NSBundle::mainBundle()
        .bundlePath()
        .to_string();
    let app = std::path::Path::new(&bundle).join("Contents/Applications/QingjianClipboard.app");
    app.is_dir().then_some(app)
}

/// 启动与每次套用配置时调：开着而它没在跑就拉起来（崩了的下次改配置时也会补上）。
pub(super) fn sync(enabled: bool) {
    if !enabled {
        return;
    }
    let running = NSRunningApplication::runningApplicationsWithBundleIdentifier(
        &NSString::from_str(BUNDLE_ID),
    );
    if !running.is_empty() {
        return;
    }
    let Some(app) = clipboard_app() else {
        tracing::warn!("clipboard app is missing from the bundle");
        return;
    };
    // -g：不切到前台，剪贴板平时只在菜单栏
    open_with_system(&["-g", &app.to_string_lossy(), "--args", "--managed"]);
}
