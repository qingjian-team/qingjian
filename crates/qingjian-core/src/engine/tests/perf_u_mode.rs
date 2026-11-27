//! u 模式的性能护栏：只在 release 下做判断，而且要显式 `--ignored` 才跑。
//!
//! ```text
//! cargo test --release -p qingjian-core --lib -- --ignored perf_u_mode
//! ```
//!
//! 为什么不放进常规测试：debug 构建慢一个数量级（同一条路 50–90 ms，release 7–9 ms）。
//! 用宽松阈值去兜 debug 等于没兜；收紧阈值又会被 debug 的抖动误报。所以这条只在 release 下
//! 判断，`[ignore]` 让它默认不跑。
//!
//! 预算给 25 ms 是按 release 实测 7–9 ms 留的三倍余量：它的作用是「别哪天又悄悄回去扫全表」，
//! 不是压榨那几毫秒 —— 想压毫秒得先有 profiler 的采样，别在这里改阈值自我安慰。

use std::time::{Duration, Instant};

use super::*;

/// 每次按键的预算。release 实测 7–9 ms，留三倍余量。
const BUDGET: Duration = Duration::from_millis(25);

/// 测几轮取中位数：单次测量受调度抖动影响太大。
const ROUNDS: usize = 15;

#[test]
#[ignore = "手动基准：只在 release 下有判断力，见文件头注释"]
fn perf_u_mode_keeps_each_key_within_budget() {
    let mut engine = engine();
    // 先热一次：笔画表第一次解析是一次性成本（release 37–45 ms），不该算进每次按键
    engine.set_input("uhszhhsn");
    let _ = engine.query().unwrap();

    let mut samples: Vec<Duration> = Vec::with_capacity(ROUNDS);
    for _ in 0..ROUNDS {
        // 每轮改一下输入，别走到「输入没变就复用上一帧」的缓存上
        engine.set_input("uhszhhs");
        engine.push('n');
        let start = Instant::now();
        let _ = engine.query().unwrap();
        samples.push(start.elapsed());
    }
    samples.sort_unstable();
    let median = samples[ROUNDS / 2];
    let worst = samples[ROUNDS - 1];

    eprintln!("u 模式每次按键：中位 {median:?} · 最慢 {worst:?}（{ROUNDS} 轮，预算 {BUDGET:?}）");

    if cfg!(debug_assertions) {
        eprintln!("这是 debug 构建，比 release 慢一个数量级，不做判断；用 --release 跑");
        return;
    }
    assert!(
        median <= BUDGET,
        "u 模式每次按键中位 {median:?} 超了预算 {BUDGET:?}（最慢 {worst:?}）—— \
         确认一下是不是又有哪条路回去扫全表了"
    );
}
