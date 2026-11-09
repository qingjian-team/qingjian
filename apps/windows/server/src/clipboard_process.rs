//! 按 `[clipboard] enabled` 拉起剪贴板进程（实验功能）。关掉时不用杀：它自己看配置退出，只开一个也由它自己保证。

use std::process::{Child, Command};
use std::sync::Mutex;

static CHILD: Mutex<Option<Child>> = Mutex::new(None);

/// 启动与配置热加载时调。
pub fn sync(enabled: bool) {
    let Ok(mut child) = CHILD.lock() else {
        return;
    };
    if !enabled {
        *child = None;
        return;
    }
    if child
        .as_mut()
        .is_some_and(|running| matches!(running.try_wait(), Ok(None)))
    {
        return;
    }
    let spawned = std::env::current_exe()
        .map(|exe| exe.with_file_name("qingjian-clipboard.exe"))
        .and_then(|exe| Command::new(exe).arg("--managed").spawn());
    match spawned {
        Ok(started) => *child = Some(started),
        Err(error) => tracing::warn!(%error, "cannot start clipboard process"),
    }
}
