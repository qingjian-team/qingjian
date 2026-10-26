//! 实验：让 P2C 直接生成整句，不经过词图，也不经过重排。
//!
//! 产品路径是「Viterbi 前几条路径 → P2C 按分重排」，天花板就是那几条路径里有没有正确答案；
//! 这把尺子绕开词图，量的是同一个模型自由生成能到哪，用来判断重排这层架构值不值得留。

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use qingjian_core::Engine;
use qingjian_neural::{CharScorer, P2c};

use super::{EvalError, collect};

/// 取前几条生成候选。
const BEAM: usize = 5;

/// 生成长度上限（字）。
const MAX_CHARS: usize = 32;

/// P2C 自由生成的评测结果。
#[derive(Debug, Default)]
pub struct GenerateReport {
    /// 评了的句子数。
    pub total: usize,

    /// 生成失败（超出模型上下文等）的句子数。
    pub failed: usize,

    /// 首选就是原句 / 原句在前五条里。
    pub top1: usize,
    pub top5: usize,

    /// 首选与原句逐字比对。
    pub chars_correct: usize,
    pub chars_total: usize,

    /// 生成耗时之和与最大值。
    pub time: Duration,
    pub slowest: Duration,
}

impl std::fmt::Display for GenerateReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let percent = |part: usize, whole: usize| {
            if whole == 0 {
                "-".to_owned()
            } else {
                format!("{:.1}%", part as f64 * 100.0 / whole as f64)
            }
        };
        writeln!(f, "P2C 自由生成（不经词图、不经重排）")?;
        writeln!(
            f,
            "句子 {:>5} 条  首选 {:>6}  前五 {:>6}  字准确率 {:>6}  生成失败 {}",
            self.total,
            percent(self.top1, self.total),
            percent(self.top5, self.total),
            percent(self.chars_correct, self.chars_total),
            self.failed,
        )?;
        if self.total > 0 {
            writeln!(
                f,
                "生成平均 {:.1} ms，最慢 {:.1} ms（束宽 {BEAM}）",
                self.time.as_secs_f64() * 1000.0 / self.total as f64,
                self.slowest.as_secs_f64() * 1000.0,
            )?;
        }
        Ok(())
    }
}

/// 在同一份句子集上跑一遍自由生成。
pub fn run(
    engine: &Engine,
    paths: &[PathBuf],
    model: &Path,
    details: Option<&Path>,
) -> Result<GenerateReport, EvalError> {
    let mut report = super::Report::default();
    let pairs = collect(engine, paths, &mut report)?;
    let scorer = CharScorer::load(model)?;
    let decoder = P2c::new(scorer.model(), scorer.vocab())
        .ok_or(qingjian_neural::NeuralError::Corrupt("模型不是 P2C"))?;
    let mut writer = details
        .map(|path| {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map(BufWriter::new)
                .map_err(|source| EvalError::Write {
                    path: path.to_owned(),
                    source,
                })
        })
        .transpose()?;
    let mut out = GenerateReport::default();
    for (index, pair) in pairs.iter().enumerate() {
        out.total += 1;
        let keys = pair.pinyin.replace('\'', "");
        let started = Instant::now();
        let generated = decoder.convert(&keys, BEAM, MAX_CHARS);
        let elapsed = started.elapsed();
        out.time += elapsed;
        out.slowest = out.slowest.max(elapsed);
        let Ok(generated) = generated else {
            out.failed += 1;
            continue;
        };
        out.chars_total += pair.text.chars().count();
        if let Some(first) = generated.first() {
            out.chars_correct += first
                .text
                .chars()
                .zip(pair.text.chars())
                .filter(|(a, b)| a == b)
                .count();
            if first.text == pair.text {
                out.top1 += 1;
            }
        }
        if generated.iter().any(|c| c.text == pair.text) {
            out.top5 += 1;
        }
        if let Some(writer) = &mut writer {
            let row = serde_json::json!({
                "text": pair.text, "pinyin": pair.pinyin,
                "generated": generated.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
                "generate_ms": elapsed.as_secs_f64() * 1000.0,
            });
            writeln!(writer, "{row}").map_err(|source| EvalError::Write {
                path: details.expect("writer has path").to_owned(),
                source,
            })?;
        }
        if (index + 1) % 500 == 0 {
            tracing::info!(done = index + 1, total = pairs.len(), "生成评测进度");
        }
    }
    if let Some(writer) = &mut writer {
        writer.flush().map_err(|source| EvalError::Write {
            path: details.expect("writer has path").to_owned(),
            source,
        })?;
    }
    Ok(out)
}
