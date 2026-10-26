//! 拿真的 P2C 模型跑一遍：拼音 → 候选，顺带量每条的耗时。
//!
//! 用法：`cargo run --release -p qingjian-neural --example p2c_try --features metal -- <模型目录> [拼音...]`
//!
//! 不给拼音就跑内置样例，并按 beam 1/3/5/8 扫一遍延迟（中位数，热身 3 次取 15 次）。

use std::path::PathBuf;
use std::time::Instant;

use qingjian_neural::{CharScorer, P2c};

const SAMPLES: &[&str] = &[
    "jiekou",
    "ganga",
    "nihoa",
    "daohanglan",
    "zhegeAPIdefanhuizhi",
    "womenmingtiankaihui",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path: PathBuf = args
        .next()
        .unwrap_or_else(|| "data/models/hanzhang-tongbian".into())
        .into();
    let keys: Vec<String> = args.collect();
    let keys: Vec<&str> = if keys.is_empty() {
        SAMPLES.to_vec()
    } else {
        keys.iter().map(String::as_str).collect()
    };

    let load = Instant::now();
    let scorer = CharScorer::load(&path)?;
    let load_ms = load.elapsed().as_secs_f64() * 1e3;
    let Some(p2c) = P2c::new(scorer.model(), scorer.vocab()) else {
        return Err("含章·通变模型的字表需要 <sep> 分隔符".into());
    };
    println!(
        "模型 {} 加载 {load_ms:.0} ms，字表 {}，设备 {:?}\n",
        path.display(),
        scorer.vocab().len(),
        scorer.model().device()
    );

    for key in &keys {
        let out = p2c.convert(key, 3, 24)?;
        println!("{key}");
        for candidate in &out {
            println!("    {:<24}{:7.2}", candidate.text, candidate.score);
        }
    }

    println!("\n延迟（中位数 ms，热身 3 次取 15 次）");
    print!("{:<22}", "拼音 / 汉字数");
    for beam in [1, 3, 5, 8] {
        print!("  beam {beam:<5}");
    }
    println!();
    for key in &keys {
        let chars = p2c
            .convert(key, 1, 24)?
            .first()
            .map_or(0, |c| c.text.chars().count());
        print!("{:<22}", format!("{key} / {chars}"));
        for beam in [1, 3, 5, 8] {
            print!(
                "  {:<10.1}",
                median(15, || {
                    p2c.convert(key, beam, 24).unwrap();
                })
            );
        }
        println!();
    }
    Ok(())
}

/// 跑 `runs` 次取中位数（毫秒），前面先热身 3 次。
fn median(runs: usize, mut run: impl FnMut()) -> f64 {
    for _ in 0..3 {
        run();
    }
    let mut times: Vec<f64> = (0..runs)
        .map(|_| {
            let started = Instant::now();
            run();
            started.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    times.sort_by(f64::total_cmp);
    times[runs / 2]
}
