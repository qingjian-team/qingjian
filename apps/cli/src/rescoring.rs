//! 异步重打分在 CLI 里的等法：壳是停顿后请求、定时器轮询，这里没有停顿，查询完直接请求并阻塞等结果，再查一次。

use std::time::{Duration, Instant};

use qingjian_core::{Engine, Query};

/// 最多来回几轮：生成的整句回来后，选词模型还要再给新的一页打分。
const ROUNDS: usize = 3;

/// 最多等多久。
const WAIT: Duration = Duration::from_secs(5);

/// 有整句路径还没拿到神经分就请求并等到分回来；返回是否等到了（等到了调用方该重新查询）。
pub fn settle(engine: &mut Engine) -> bool {
    if !engine.rescoring_pending() || !engine.request_rescoring() {
        return false;
    }
    let started = Instant::now();
    while !engine.poll_rescoring() {
        if started.elapsed() > WAIT {
            tracing::warn!("等重打分超时");
            return false;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    true
}

/// 一直等到没有后台请求在等（最多 [`ROUNDS`] 轮），返回最后一次查询的结果。
pub fn settled(engine: &mut Engine, mut query: Query) -> Query {
    for _ in 0..ROUNDS {
        if !settle(engine) {
            break;
        }
        match engine.query() {
            Ok(next) => query = next,
            Err(_) => break,
        }
    }
    query
}
