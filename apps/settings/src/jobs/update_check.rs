//! 「立即检查」更新：在自己的线程里查（下载索引、验签、写 `update.json`），查完叫醒窗口。
//! 自动检查由输入法每天做，结果也在 `update.json`，这里打开时读一次。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bamboo_slip::Waker;
use qingjian_platform::UpdateConfig;
use qingjian_update::{Available, Checker, UpdateError, UpdateState};

/// 本程序与输入法同一个版本号，按它判断有没有新版。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `None` 是本地开发版，不查。
type Outcome = Option<Result<UpdateState, UpdateError>>;

pub struct UpdateCheck {
    path: Option<PathBuf>,

    /// 上次（输入法或这里）查到的结果。
    pub state: UpdateState,

    result: Arc<Mutex<Option<Outcome>>>,

    running: bool,
}

impl UpdateCheck {
    pub fn load() -> Self {
        let path = crate::system::paths::user_dir().map(|dir| dir.join("update.json"));
        let state = path.as_deref().map(UpdateState::load).unwrap_or_default();
        Self {
            path,
            state,
            result: Arc::default(),
            running: false,
        }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    /// 要提示的新版本。
    pub fn available(&self, config: &UpdateConfig) -> Option<Available> {
        Checker::available_in(&self.state, VERSION, config)
    }

    pub fn start(&mut self, config: &UpdateConfig, waker: Option<Waker>) -> Result<(), String> {
        if Checker::is_dev_version(VERSION) {
            return Err("本地开发版，不检查更新".to_owned());
        }
        let path = self.path.clone().ok_or("找不到数据目录")?;
        self.running = true;
        let config = config.clone();
        let result = Arc::clone(&self.result);
        std::thread::spawn(move || {
            let outcome = Checker::check_blocking(&path, VERSION, &config);
            if let Ok(mut slot) = result.lock() {
                *slot = Some(outcome);
            }
            if let Some(waker) = waker {
                waker.wake();
            }
        });
        Ok(())
    }

    /// 结果到了就收下，说成一句话。
    pub fn finish(&mut self, config: &UpdateConfig) -> Option<String> {
        let outcome = self.result.lock().ok()?.take()?;
        self.running = false;
        Some(match outcome {
            None => "本地开发版，不检查更新".to_owned(),
            Some(Err(error)) => {
                tracing::warn!(%error, "检查更新失败");
                format!("检查失败：{error}")
            }
            Some(Ok(state)) => {
                self.state = state;
                match self.available(config) {
                    Some(found) => format!("有新版本 {}", found.version),
                    None => "已是最新版本".to_owned(),
                }
            }
        })
    }
}
