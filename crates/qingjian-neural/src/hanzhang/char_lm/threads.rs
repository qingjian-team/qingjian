//! 模型推理专用的线程池：Q8 矩阵乘与 candle 的 CPU 算子都走 rayon，缺省的全局池有几个逻辑核开几个线程，
//! 一次只算一两百毫秒的小模型摊到二十个线程上并不更快，多出来的 CPU 几乎全是线程空转等活。
//! 数字见 `docs/notes/crate-notes.md` 的 qingjian-neural 一节。

use std::sync::OnceLock;

use rayon::{ThreadPool, ThreadPoolBuilder};

/// 推理最多用几个线程。
const MAX_THREADS: usize = 4;

/// 在推理线程池里跑 `work`；池建不起来（不该发生）就在当前线程跑。
pub(super) fn install<R: Send>(work: impl FnOnce() -> R + Send) -> R {
    static POOL: OnceLock<Option<ThreadPool>> = OnceLock::new();
    let pool = POOL.get_or_init(|| {
        let available = std::thread::available_parallelism().map_or(1, usize::from);
        ThreadPoolBuilder::new()
            .num_threads(available.min(MAX_THREADS))
            .thread_name(|index| format!("neural-{index}"))
            .build()
            .ok()
    });
    match pool {
        Some(pool) => pool.install(work),
        None => work(),
    }
}
