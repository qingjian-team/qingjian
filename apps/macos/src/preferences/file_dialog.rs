use objc2::MainThreadMarker;
use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
use objc2_foundation::NSString;
use std::path::PathBuf;

/// 弹系统的打开文件对话框选一个词库文件（TSV / Rime yaml / .qj）。用户取消返回 `None`。
/// 模态运行，只在偏好设置窗口打开时调用（那时进程已是 Accessory，能出对话框）。
pub fn choose_dictionary_file() -> Option<PathBuf> {
    let mtm = MainThreadMarker::new()?;
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(false);
    panel.setAllowsMultipleSelection(false);
    panel.setMessage(Some(&NSString::from_str(
        "选择要导入的词库：青简 TSV、Rime .dict.yaml 或 .qj",
    )));
    panel.setPrompt(Some(&NSString::from_str("导入")));
    if panel.runModal() != NSModalResponseOK {
        return None;
    }
    let urls = panel.URLs();
    let url = urls.firstObject()?;
    let path = url.path()?;
    Some(PathBuf::from(path.to_string()))
}
