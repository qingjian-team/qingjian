//! 本地整句模型：后台加载、停键防抖后请求重排、结果到了重查重画。
//!
//! 与 macOS 壳同一套引擎流程(`rescoring_pending` → `request_rescoring` → `poll_rescoring`),
//! 定时驱动不同：macOS 用 NSTimer，这里由 shim 的 fcitx5 TimeEvent 每 20ms 调一次
//! [`Host::model_poll`]，防抖与超时的状态机都在本文件。

use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Instant;

use qingjian_neural::{CharScorer, NeuralError};

use super::Host;

/// 后台加载线程的回执通道。
pub(super) type ModelLoader = Receiver<Result<CharScorer, NeuralError>>;

/// 停键多久才请求重排：比一般的击键间隔短，连着敲时不请求。
const DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(80);

/// 最长等多久；后台线程卡住时兜底。
const MAX_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

impl Host {
    /// 在后台线程加载模型并预热（第一次前向要编译内核，几百毫秒），加载完由
    /// [`Self::attach_loaded_model`] 接上。没有模型文件就什么都不做。
    pub(super) fn load_local_model(&mut self) {
        if self.model_loader.is_some() || self.engine.has_sentence_scorer() {
            return;
        }
        // 用户自己的模型放 model/，盖过随包层 dist/model/（分层见 paths.rs）。
        let Some(path) = qingjian_neural::find_model(&self.data_dir.join("model"))
            .or_else(|| qingjian_neural::find_model(&self.data_dir.join("dist/model")))
        else {
            tracing::info!("没有本地整句模型文件,不重排");
            return;
        };
        let (tx, rx) = channel::<Result<CharScorer, NeuralError>>();
        let spawned = std::thread::Builder::new()
            .name("qingjian-model-load".to_owned())
            .spawn(move || {
                let started = Instant::now();
                let loaded = CharScorer::load(&path).and_then(|scorer| {
                    scorer.score("", &["的"])?;
                    Ok(scorer)
                });
                if loaded.is_ok() {
                    tracing::info!(
                        path = %path.display(),
                        total_ms = started.elapsed().as_millis(),
                        "本地整句模型已加载并预热"
                    );
                }
                let _ = tx.send(loaded);
            });
        match spawned {
            Ok(_) => self.model_loader = Some(rx),
            Err(error) => tracing::warn!(%error, "起不了模型加载线程,本地整句模型不用"),
        }
    }

    /// 加载线程有结果了就接到 Engine 上；定时与查询路径顺手看一眼，不阻塞。
    pub(super) fn attach_loaded_model(&mut self) {
        let Some(rx) = &self.model_loader else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(scorer)) => {
                self.engine
                    .set_async_sentence_scorer(Some(Box::new(scorer)));
                self.model_loader = None;
            }
            Ok(Err(error)) => {
                tracing::warn!(%error, "本地整句模型加载失败,不重排");
                self.model_loader = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.model_loader = None,
        }
    }

    /// 卸掉模型（配置关掉）。
    pub(super) fn unload_local_model(&mut self) {
        self.model_loader = None;
        self.engine.set_async_sentence_scorer(None);
        self.rescore_deadline = None;
        self.rescore_since = None;
    }

    /// 每次查询之后：有整句路径等着打分就（重新）起防抖计时——又敲了一键就重头等。
    pub(super) fn schedule_rescoring(&mut self) {
        if self.engine.rescoring_pending() {
            self.rescore_deadline = Some(Instant::now() + DEBOUNCE);
        }
    }

    /// 定时驱动（shim 每 20ms 调一次）。返回位掩码：
    /// bit0 = 重排结果换了排序，shim 要重画面板；bit1 = 还有事在等，继续定时。
    pub fn model_poll(&mut self) -> u32 {
        let had_scorer = self.engine.has_sentence_scorer();
        self.attach_loaded_model();
        if !had_scorer && self.engine.has_sentence_scorer() && self.composing() {
            // 模型刚在后台接上：加载窗口里敲出的这一轮查询从没见过打分器，
            // rescoring_pending 一直是 false，不补查一次这一轮就永远错过重排。
            // （按键路径不用补：refresh 开头就 attach，随后的查询自带打分器。）
            // 用户已翻页或动过高亮就不打扰（与下面 poll_rescoring 的克制同款）。
            if self.page == 0 && !self.navigated {
                self.refresh();
            }
        }
        if !self.composing() {
            self.rescore_deadline = None;
            self.rescore_since = None;
            return self.pending_bit();
        }
        if let Some(deadline) = self.rescore_deadline
            && Instant::now() >= deadline
        {
            self.rescore_deadline = None;
            if self.engine.request_rescoring() {
                self.rescore_since = Some(Instant::now());
            }
        }
        if let Some(since) = self.rescore_since {
            if self.engine.poll_rescoring() {
                self.rescore_since = None;
                // 用户已翻页或动过高亮就只留着分不动画面（与 macOS 同款克制）。
                if self.page == 0 && !self.navigated {
                    // refresh 可能又攒下新的整句路径（重新起防抖），重画位之外还得带续轮位。
                    self.refresh();
                    return 0b01 | self.pending_bit();
                }
            } else if since.elapsed() > MAX_WAIT {
                // 等太久多半是前文变了、结果作废；真卡住也只是这轮不重排。
                tracing::debug!("等本地整句模型超时,本轮不重排");
                self.rescore_since = None;
            }
        }
        self.pending_bit()
    }

    /// 还有事在等（加载线程、防抖计时、等分）就带上「继续定时」位。
    fn pending_bit(&self) -> u32 {
        if self.model_loader.is_some()
            || self.rescore_deadline.is_some()
            || self.rescore_since.is_some()
        {
            0b10
        } else {
            0
        }
    }
}
