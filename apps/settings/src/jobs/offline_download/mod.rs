//! 离线翻译模型的下载任务；窗口退出或点取消时停止，核对成功后才启用。

mod fetch;
mod shared;

use std::sync::{Arc, atomic::Ordering};

use bamboo_slip::Waker;

use self::shared::Shared;
use crate::jobs::model_download::Stage;

#[derive(Default)]
pub struct OfflineDownload {
    current: Option<Arc<Shared>>,
}

impl OfflineDownload {
    pub fn stage(&self) -> Option<Stage> {
        self.current.as_ref()?.stage.lock().ok().map(|s| s.clone())
    }

    pub fn busy(&self) -> bool {
        matches!(
            self.stage(),
            Some(Stage::Downloading { .. } | Stage::Installing)
        )
    }

    pub fn start(&mut self, waker: Option<Waker>) -> Result<(), String> {
        if self.busy() {
            return Err("正在下载翻译模型".into());
        }
        let user = crate::system::paths::user_dir().ok_or("找不到数据目录")?;
        let shared = Arc::new(Shared::new(waker));
        let state = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("qingjian-translation-download".into())
            .spawn(move || {
                let result = fetch::run(&user, &state);
                state.set(match result {
                    Ok(()) => Stage::Done,
                    Err(error) => Stage::Failed(error),
                });
            })
            .map_err(|error| error.to_string())?;
        self.current = Some(shared);
        Ok(())
    }

    pub fn cancel(&self) {
        if let Some(shared) = &self.current {
            shared.cancel.store(true, Ordering::Release);
        }
    }

    pub fn finished(&mut self) -> bool {
        if matches!(self.stage(), Some(Stage::Done)) {
            self.current = None;
            true
        } else {
            false
        }
    }
}

impl Drop for OfflineDownload {
    fn drop(&mut self) {
        self.cancel();
    }
}
