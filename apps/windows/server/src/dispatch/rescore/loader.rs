//! 模型的后台加载：加载并预热要几百毫秒到几秒，不能挡住按键；加载完由 Router 在下一次消息时接上。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use qingjian_neural::{CharScorer, NeuralError};

/// 一次进行中的加载。
pub(crate) struct ModelLoader {
    result: Receiver<Result<CharScorer, NeuralError>>,
}

/// 一次加载的结局。
pub(super) enum Loaded {
    /// 还在加载。
    Pending,

    /// 加载好了（或失败了，`Err`）。装箱：模型结构体比另两个变体大得多。
    Done(Box<Result<CharScorer, NeuralError>>),

    /// 加载线程没了（起不来 / panic）。
    Gone,
}

impl ModelLoader {
    /// 起线程加载 `dir` 下的三件套并预热一次；线程起不来返回 `None`。
    pub(super) fn spawn(dir: &Path) -> Option<Self> {
        let dir: PathBuf = dir.to_path_buf();
        let (tx, result) = channel();
        let spawned = std::thread::Builder::new()
            .name("qingjian-model-load".to_owned())
            .spawn(move || {
                let started = std::time::Instant::now();
                let loaded = CharScorer::load(&dir).and_then(|scorer| {
                    scorer.score("", &["的"])?;
                    Ok(scorer)
                });
                if loaded.is_ok() {
                    tracing::info!(
                        dir = %dir.display(),
                        total_ms = started.elapsed().as_millis(),
                        "本地整句模型已加载并预热"
                    );
                }
                let _ = tx.send(loaded);
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
            Ok(result) => Loaded::Done(Box::new(result)),
            Err(TryRecvError::Empty) => Loaded::Pending,
            Err(TryRecvError::Disconnected) => Loaded::Gone,
        }
    }
}
