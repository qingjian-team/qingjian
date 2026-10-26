//! P2C 解码延迟 spike：用现有字级模型模拟「拼音 → 汉字」解码器的 beam search，量一次整句重解码要多久。
//!
//! 现有模型不带拼音条件，但 P2C 解码的成本主导项是「beam 条并成一批、逐字推进的前向 + 每步取回 CPU 挑 top-k」，
//! 与条件从哪来无关，所以拿它当代理量级是成立的（见报告里的偏差说明）。
//!
//! 用法：`cargo run --release -p qingjian-neural --example p2c_bench --features metal -- [模型路径]`

use std::path::PathBuf;
use std::time::Instant;

use candle_core::Tensor;
use qingjian_neural::{CharLm, CharScorer};

/// 前文取样文本，按需截断到指定字数。
const CONTEXT: &str = "这个方案的问题在于每敲一键都要把整句重新解码一遍，候选窗要等模型算完才能出，\
所以延迟不是体验好坏的问题而是能不能用的问题，必须先量清楚再决定要不要换架构，不然训练白训。";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/models/hanzhang-zhiwei".into())
        .into();

    let load = Instant::now();
    let scorer = CharScorer::load(&path)?;
    let load_ms = load.elapsed().as_secs_f64() * 1e3;
    let model = scorer.model();
    let vocab_len = scorer.vocab().len();
    println!(
        "模型 {} 加载 {load_ms:.0} ms，字表 {vocab_len}，设备 {:?}",
        path.display(),
        model.device()
    );

    let chars: Vec<char> = CONTEXT.chars().collect();
    let context_ids = |n: usize| -> Vec<u32> {
        let text: String = chars.iter().take(n.max(1)).collect();
        scorer.vocab().encode(&text)
    };

    // 基线：现在线上的重排（前文 64 字，8 条候选各自打分）
    let ctx: String = chars.iter().take(64).collect();
    let texts = [
        "候选生成",
        "后选生成",
        "候选声称",
        "后选声称",
        "候选生",
        "候选",
        "后选",
        "生成",
    ];
    let (med, p95) = bench(20, || {
        scorer.score(&ctx, &texts).unwrap();
    });
    // 对拍：改动过 attention 的返回值，这几个分数应与 README 记的基线一致
    let probe = ["上海", "伤害", "参数", "残数", "错误", "措施"];
    let a = scorer.score("我今天想去", &probe[..2])?;
    let b = scorer.score("这个接口的", &probe[2..4])?;
    let c = scorer.score("编译器报了一个", &probe[4..])?;
    println!(
        "对拍 上海 {:.1} / 伤害 {:.1}；参数 {:.1} / 残数 {:.1}；错误 {:.1} / 措施 {:.1}",
        a[0], a[1], b[0], b[1], c[0], c[1]
    );
    println!("\n== 基线：现有重排（前文 64 字 × 8 条候选打分）==");
    println!("  中位 {med:6.1} ms   p95 {p95:6.1} ms");

    println!("\n== 每键增量：前文缓存上推进 1 个 token ==");
    for prefix in [0usize, 32, 64] {
        let ids = context_ids(prefix + 1);
        let (head, tail) = ids.split_at(ids.len() - 1);
        let cache = model.prefix_cache(head)?;
        let idx = Tensor::from_vec(tail.to_vec(), (1, 1), model.device())?;
        let (med, p95) = bench(30, || {
            model.step(&cache, &idx).unwrap();
        });
        println!("  前文 {prefix:3} 字   中位 {med:6.2} ms   p95 {p95:6.2} ms");
    }

    println!("\n== 整句重解码：beam search（每敲一键都要重来一次）==");
    println!("  前文  beam  字数    中位 ms     p95 ms");
    for prefix in [0usize, 64] {
        for beam in [1usize, 4, 8, 16] {
            for steps in [4usize, 8, 16, 24] {
                let ids = context_ids(prefix + 1);
                let (med, p95) = bench(10, || {
                    decode(model, &ids, beam, steps, vocab_len).unwrap();
                });
                println!("  {prefix:4}  {beam:4}  {steps:4}   {med:8.1}   {p95:8.1}");
            }
        }
    }
    println!("\n== 成本拆解：前文 64 字，看每步取回 CPU 挑 top-k 占多少 ==");
    println!("  beam  字数         只前向   +取回 CPU     完整解码");
    let ids = context_ids(65);
    for (beam, steps) in [(4usize, 8usize), (8, 16), (8, 24)] {
        let mut cols = [0.0f64; 3];
        for (i, col) in cols.iter_mut().enumerate() {
            let stage = i as u8;
            *col = bench(10, || {
                decode_stage(model, &ids, beam, steps, stage).unwrap();
            })
            .0;
        }
        println!(
            "  {beam:4}  {steps:4}   {:10.1}  {:10.1}  {:10.1}",
            cols[0], cols[1], cols[2]
        );
    }
    Ok(())
}

/// 跑 `runs` 次取中位数与 p95（毫秒），前面另跑 3 次热身（Metal 首次要编译内核）。
fn bench(runs: usize, mut f: impl FnMut()) -> (f64, f64) {
    for _ in 0..3 {
        f();
    }
    let mut ms: Vec<f64> = (0..runs)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    ms.sort_by(f64::total_cmp);
    let p95 = ms[(runs * 95 / 100).min(runs - 1)];
    (ms[runs / 2], p95)
}

/// 一次完整的 beam search 解码：前文最后一个 token 起手，逐字推进 `steps` 步。
fn decode(
    model: &CharLm,
    prefix: &[u32],
    beam: usize,
    steps: usize,
    vocab_len: usize,
) -> candle_core::Result<()> {
    let device = model.device();
    let (head, tail) = prefix.split_at(prefix.len() - 1);
    let cache = model.prefix_cache(head)?;
    let idx = Tensor::from_vec(tail.to_vec(), (1, 1), device)?;

    // 第一步只有一条，挑出 beam 个起点后把缓存复制成 beam 行
    let (log_probs, cache) = model.step(&cache, &idx)?;
    let row = log_probs.to_vec2::<f32>()?;
    let picked = merge_top(&row, &[0.0], beam);
    let mut scores: Vec<f32> = picked.iter().map(|p| p.0).collect();
    let mut tokens: Vec<u32> = picked.iter().map(|p| p.1).collect();
    let mut cache = cache.select(&Tensor::from_vec(vec![0u32; beam], beam, device)?)?;

    for _ in 1..steps {
        let idx = Tensor::from_vec(tokens.clone(), (beam, 1), device)?;
        let (log_probs, next) = model.step(&cache, &idx)?;
        let rows = log_probs.to_vec2::<f32>()?;
        let picked = merge_top(&rows, &scores, beam);
        scores = picked.iter().map(|p| p.0).collect();
        tokens = picked.iter().map(|p| p.1).collect();
        let order: Vec<u32> = picked.iter().map(|p| p.2).collect();
        cache = next.select(&Tensor::from_vec(order, beam, device)?)?;
    }
    debug_assert!(tokens.iter().all(|&t| (t as usize) < vocab_len));
    Ok(())
}

/// 在 `beam × vocab` 个延长里挑分数最高的 `k` 个，返回（累计分, token, 来自第几条）。
/// 线性扫描 + 小数组插入，不给中间结果分配大向量——真实解码器也该这么写。
fn merge_top(rows: &[Vec<f32>], base: &[f32], k: usize) -> Vec<(f32, u32, u32)> {
    let mut best: Vec<(f32, u32, u32)> = Vec::with_capacity(k + 1);
    for (b, row) in rows.iter().enumerate() {
        let carry = base.get(b).copied().unwrap_or(0.0);
        for (token, &lp) in row.iter().enumerate() {
            let score = carry + lp;
            if best.len() == k && score <= best[k - 1].0 {
                continue;
            }
            let at = best.partition_point(|p| p.0 >= score);
            best.insert(at, (score, token as u32, b as u32));
            best.truncate(k);
        }
    }
    best
}

/// 同一条解码路径的三档成本：`stage` 0 只做前向、1 再把 logits 取回 CPU、2 是完整的挑 top-k 加重排缓存。
/// 0 和 1 都在最后取一次值强制同步，免得 Metal 的异步队列把时间藏起来。
fn decode_stage(
    model: &CharLm,
    prefix: &[u32],
    beam: usize,
    steps: usize,
    stage: u8,
) -> candle_core::Result<()> {
    if stage >= 2 {
        return decode(model, prefix, beam, steps, usize::MAX);
    }
    let device = model.device();
    let (head, tail) = prefix.split_at(prefix.len() - 1);
    let cache = model.prefix_cache(head)?;
    let idx = Tensor::from_vec(tail.to_vec(), (1, 1), device)?;
    let (log_probs, cache) = model.step(&cache, &idx)?;
    let mut last = log_probs;
    let mut cache = cache.select(&Tensor::from_vec(vec![0u32; beam], beam, device)?)?;
    let idx = Tensor::from_vec(vec![1u32; beam], (beam, 1), device)?;
    for _ in 1..steps {
        let (log_probs, next) = model.step(&cache, &idx)?;
        if stage >= 1 {
            log_probs.to_vec2::<f32>()?;
        }
        last = log_probs;
        cache = next;
    }
    last.to_vec2::<f32>()?;
    Ok(())
}
