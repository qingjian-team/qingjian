//! 「实验功能」里下载模型：在自己的线程里下，进度与结果放在共享的槽里，有变化就叫醒窗口。
//! 一次只下一个；关掉设置窗口下载也就停了，下次从头下。

mod fetch;
mod manifest;
mod pack;

pub use pack::Pack;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bamboo_slip::Waker;
use qingjian_platform::LocalModel;

/// 正在下的这个走到哪了。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    Downloading {
        done: u64,
        total: u64,
    },
    /// 下完、核对过，在解压换进模型目录。
    Installing,
    Done,
    Failed(String),
}

/// 下载线程与窗口共用：线程写进度、窗口读。
struct Shared {
    stage: Mutex<Stage>,
    cancel: AtomicBool,
    waker: Option<Waker>,
}

impl Shared {
    fn set(&self, stage: Stage) {
        if let Ok(mut slot) = self.stage.lock() {
            *slot = stage;
        }
        if let Some(waker) = &self.waker {
            waker.wake();
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

#[derive(Default)]
pub struct ModelDownload {
    current: Option<(LocalModel, Arc<Shared>)>,
}

impl ModelDownload {
    /// 正在下（或刚下完、刚失败）的模型与它走到哪了。
    pub fn stage(&self) -> Option<(LocalModel, Stage)> {
        let (model, shared) = self.current.as_ref()?;
        let stage = shared.stage.lock().ok()?.clone();
        Some((*model, stage))
    }

    /// 有一个还在下或在解压。
    pub fn busy(&self) -> bool {
        matches!(
            self.stage(),
            Some((_, Stage::Downloading { .. } | Stage::Installing))
        )
    }

    pub fn start(&mut self, model: LocalModel, waker: Option<Waker>) -> Result<(), String> {
        if self.busy() {
            return Err("正在下载另一个模型".to_owned());
        }
        let pack = Pack::of(model).ok_or("这个模型不用下载")?;
        let user_dir = crate::system::paths::user_dir().ok_or("找不到数据目录")?;
        let shared = Arc::new(Shared {
            stage: Mutex::new(Stage::Downloading { done: 0, total: 0 }),
            cancel: AtomicBool::new(false),
            waker,
        });
        let worker = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("qingjian-model-download".to_owned())
            .spawn(move || {
                let stage = match fetch::run(pack, &user_dir, &worker) {
                    Ok(()) => {
                        tracing::info!(tag = pack.tag, "模型已下载安装");
                        Stage::Done
                    }
                    Err(error) => {
                        tracing::warn!(tag = pack.tag, error, "模型下载失败");
                        Stage::Failed(error)
                    }
                };
                worker.set(stage);
            })
            .map_err(|error| format!("起不了下载线程：{error}"))?;
        self.current = Some((model, shared));
        Ok(())
    }

    pub fn cancel(&self) {
        if let Some((_, shared)) = &self.current {
            shared.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// 下完的收掉、返回它（卡片换成开关）；取消的也收掉。失败的留着，卡片上说原因、给「重试」。
    pub fn finished(&mut self) -> Option<LocalModel> {
        let (model, stage) = self.stage()?;
        match stage {
            Stage::Done => {
                self.current = None;
                Some(model)
            }
            Stage::Failed(error) if error == fetch::CANCELLED => {
                self.current = None;
                None
            }
            _ => None,
        }
    }
}
