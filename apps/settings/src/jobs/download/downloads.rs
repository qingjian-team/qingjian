//! 下载任务本身：起线程、报进度、取消、收结果。

use std::sync::Arc;
use std::sync::atomic::Ordering;

use bamboo_slip::Waker;

use super::fetch::{self, CANCELLED};
use super::shared::Shared;
use super::{Item, Stage, Target};

use crate::i18n::{tr, tr_args};

#[derive(Default)]
pub struct Downloads {
    current: Option<(Item, Arc<Shared>)>,
}

impl Downloads {
    /// 正在下（或刚下完、刚失败）的是哪个、走到哪了。
    pub fn stage(&self) -> Option<(Item, Stage)> {
        let (item, shared) = self.current.as_ref()?;
        let stage = shared.stage.lock().ok()?.clone();
        Some((item.clone(), stage))
    }

    /// `item` 的进度；下的不是它就是 `None`。
    pub fn stage_of(&self, item: &Item) -> Option<Stage> {
        self.stage()
            .filter(|(current, _)| current == item)
            .map(|(_, stage)| stage)
    }

    /// 有一个还在下或在解压。
    pub fn busy(&self) -> bool {
        matches!(
            self.stage(),
            Some((_, Stage::Downloading { .. } | Stage::Installing))
        )
    }

    pub fn start(&mut self, target: Target, waker: Option<Waker>) -> Result<(), String> {
        if self.busy() {
            return Err(tr("download.busy").to_owned());
        }
        let user_dir = crate::system::paths::user_dir().ok_or(tr("common.no_data_dir"))?;
        let shared = Arc::new(Shared::new(waker));
        let worker = Arc::clone(&shared);
        let item = target.item.clone();
        std::thread::Builder::new()
            .name("qingjian-download".to_owned())
            .spawn(move || {
                let stage = match fetch::run(&target, &user_dir, &worker) {
                    Ok(()) => {
                        tracing::info!(tag = target.tag, "download installed");
                        Stage::Done
                    }
                    Err(error) => {
                        tracing::warn!(tag = target.tag, error, "download failed");
                        Stage::Failed(error)
                    }
                };
                worker.set(stage);
            })
            .map_err(|error| tr_args("download.no_thread", &[("error", &error)]))?;
        self.current = Some((item, shared));
        Ok(())
    }

    pub fn cancel(&self) {
        if let Some((_, shared)) = &self.current {
            shared.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// 下完的收掉、返回它；取消的也收掉。失败的留着，界面上说原因、给「重试」。
    pub fn finished(&mut self) -> Option<Item> {
        let (item, stage) = self.stage()?;
        match stage {
            Stage::Done => {
                self.current = None;
                Some(item)
            }
            Stage::Failed(error) if error == CANCELLED => {
                self.current = None;
                None
            }
            _ => None,
        }
    }
}
