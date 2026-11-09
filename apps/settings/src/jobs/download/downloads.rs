//! 下载任务本身：起线程、报进度、取消、收结果。一次只下一个，别的排队，前一个下完（或失败、取消）接着下下一个。

use std::collections::VecDeque;
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

    /// 排着队的，按点的先后。
    queue: VecDeque<(Target, Option<Waker>)>,

    /// 失败了的和原因：后面的接着下时，失败那一行仍显示原因与「重试」。
    failures: Vec<(Item, String)>,
}

impl Downloads {
    /// 正在下（或刚下完、刚失败）的是哪个、走到哪了。
    pub fn stage(&self) -> Option<(Item, Stage)> {
        let (item, shared) = self.current.as_ref()?;
        let stage = shared.stage.lock().ok()?.clone();
        Some((item.clone(), stage))
    }

    /// `item` 的进度：正在下的、排着队的、失败过的；都不是为 `None`。
    pub fn stage_of(&self, item: &Item) -> Option<Stage> {
        if let Some((current, stage)) = self.stage()
            && current == *item
        {
            return Some(stage);
        }
        if self.queue.iter().any(|(target, _)| target.item == *item) {
            return Some(Stage::Queued);
        }
        self.failures
            .iter()
            .find(|(failed, _)| failed == item)
            .map(|(_, error)| Stage::Failed(error.clone()))
    }

    /// 有一个还在下或在解压。
    pub fn busy(&self) -> bool {
        matches!(
            self.stage(),
            Some((_, Stage::Downloading { .. } | Stage::Installing))
        )
    }

    /// 下这一项：没有别的在下就马上开始，有就排到队尾；已经在下或在排队的不重复加。
    pub fn start(&mut self, target: Target, waker: Option<Waker>) -> Result<(), String> {
        let item = &target.item;
        let running = self.busy() && self.current.as_ref().is_some_and(|(cur, _)| cur == item);
        if running || self.queue.iter().any(|(queued, _)| queued.item == *item) {
            return Ok(());
        }
        self.failures.retain(|(failed, _)| failed != item);
        if self.busy() {
            self.queue.push_back((target, waker));
            return Ok(());
        }
        self.spawn(target, waker)
    }

    fn spawn(&mut self, target: Target, waker: Option<Waker>) -> Result<(), String> {
        let downloads = super::downloads_dir().ok_or(tr("common.no_data_dir"))?;
        let shared = Arc::new(Shared::new(waker));
        let worker = Arc::clone(&shared);
        let item = target.item.clone();
        std::thread::Builder::new()
            .name("qingjian-download".to_owned())
            .spawn(move || {
                let stage = match fetch::run(&target, &downloads, &worker) {
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

    /// 取消一项：排着队的直接拿掉，正在下的让线程停下。
    pub fn cancel(&mut self, item: &Item) {
        let before = self.queue.len();
        self.queue.retain(|(target, _)| target.item != *item);
        if self.queue.len() != before {
            return;
        }
        if let Some((current, shared)) = &self.current
            && current == item
        {
            shared.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// 下完的收掉、返回它；取消的也收掉；失败的记下原因。收掉之后接着下队里的下一个。
    pub fn finished(&mut self) -> Option<Item> {
        let (item, stage) = self.stage()?;
        let done = match stage {
            Stage::Done => Some(item),
            Stage::Failed(error) => {
                if error != CANCELLED {
                    self.failures.push((item, error));
                }
                None
            }
            Stage::Downloading { .. } | Stage::Installing | Stage::Queued => return None,
        };
        self.current = None;
        while let Some((target, waker)) = self.queue.pop_front() {
            let item = target.item.clone();
            match self.spawn(target, waker) {
                Ok(()) => break,
                Err(error) => self.failures.push((item, error)),
            }
        }
        done
    }
}
