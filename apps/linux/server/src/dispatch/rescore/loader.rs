//! 模型的后台加载：加载并预热要几百毫秒到几秒，不能挡住按键；加载完由 Router 在下一次消息或 tick 时接上。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use qingjian_core::sentence::SentenceScorer;
use qingjian_neural::{CharScorer, NeuralError, P2cScorer};

/// 一次进行中的加载。
pub(crate) struct ModelLoader {
    result: Receiver<Result<Box<dyn SentenceScorer>, NeuralError>>,
}

/// 一次加载的结局。
pub(super) enum Loaded {
    /// 还在加载。
    Pending,

    /// 加载好了（或失败了）。
    Done(Result<Box<dyn SentenceScorer>, NeuralError>),

    /// 加载线程没了（起不来 / panic）。
    Gone,
}

impl ModelLoader {
    /// 起线程加载 `path`（`.qjm` 或三件套目录）并预热一次；线程起不来返回 `None`。
    pub(super) fn spawn(path: &Path) -> Option<Self> {
        let path: PathBuf = path.to_path_buf();
        let (tx, result) = channel();
        let spawned = std::thread::Builder::new()
            .name("qingjian-model-load".to_owned())
            .spawn(move || {
                let started = std::time::Instant::now();
                let loaded = CharScorer::load(&path).and_then(|scorer| {
                    if scorer.vocab().sep().is_some() {
                        let p2c = P2cScorer::new(scorer).expect("P2C vocab checked above");
                        p2c.0.score_p2c("ni", &["你"])?;
                        Ok((Box::new(p2c) as Box<dyn SentenceScorer>, "含章·通变"))
                    } else {
                        scorer.score("", &["的"])?;
                        Ok((Box::new(scorer) as Box<dyn SentenceScorer>, "含章·知微"))
                    }
                });
                if let Ok((_, kind)) = &loaded {
                    tracing::info!(
                        path = %path.display(),
                        kind,
                        total_ms = started.elapsed().as_millis(),
                        "本地整句模型已加载并预热"
                    );
                }
                let _ = tx.send(loaded.map(|(scorer, _)| scorer));
            });
        match spawned {
            Ok(_) => Some(Self { result }),
            Err(error) => {
                tracing::warn!(%error, "起不了模型加载线程，本地整句模型不用");
                None
            }
        }
    }

    /// 看一眼有没有结果，不阻塞。
    pub(super) fn poll(&self) -> Loaded {
        match self.result.try_recv() {
            Ok(result) => Loaded::Done(result),
            Err(TryRecvError::Empty) => Loaded::Pending,
            Err(TryRecvError::Disconnected) => Loaded::Gone,
        }
    }
}
