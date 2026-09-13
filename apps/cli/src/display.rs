//! 候选与耗时的终端排版。

use std::time::{Duration, Instant};

use qingjian_core::{Candidate, Engine, Query};

/// 等联想结果的轮询间隔与上限。CLI 是同步工具，等一等无妨；输入法里是定时器轮询、不等。
const PREDICTION_POLL: Duration = Duration::from_millis(20);
const PREDICTION_WAIT: Duration = Duration::from_secs(8);

/// 查询并打印一段拼音的候选、译文和耗时。返回查询结果供后续上屏用。
///
/// 输入里的 `|` 表示光标位置（`ni|hao`），用来验证光标停在中间时的候选。
pub fn show(engine: &mut Engine, input: &str, limit: usize) -> Option<Query> {
    let cursor = input.find('|');
    engine.set_input(&input.replace('|', ""));
    if let Some(cursor) = cursor {
        engine.move_cursor_home();
        for _ in 0..cursor {
            engine.move_cursor_right();
        }
    }
    let mut query = match engine.query() {
        // 异步重打分：等后台的分回来再查一次，输出的就是重排后的
        Ok(query) if crate::rescoring::settle(engine) => engine.query().unwrap_or(query),
        Ok(query) => query,
        Err(error) => {
            println!("  {error}");
            return None;
        }
    };
    let report = engine.annotate(&mut query.candidates);

    let segmentations: Vec<String> = query
        .segmentations
        .iter()
        .map(ToString::to_string)
        .collect();
    if query.tail.is_empty() {
        println!("  切分: {}", segmentations.join(" | "));
    } else {
        println!(
            "  切分: {}  未切分尾部: {}",
            segmentations.join(" | "),
            query.tail
        );
    }
    if let Some(correction) = &query.correction {
        println!(
            "  纠正: {} → {}",
            correction.original, correction.segmentation
        );
    }
    if query.candidates.items.is_empty() {
        println!("  （无候选）");
    }
    let width = query
        .candidates
        .items
        .iter()
        .take(limit)
        .map(|c| display_width(&c.text))
        .max()
        .unwrap_or(0);
    for (index, candidate) in query.candidates.items.iter().enumerate().take(limit) {
        println!("  {:>2}. {}", index + 1, format_candidate(candidate, width));
    }
    let hidden = query.candidates.items.len().saturating_sub(limit);
    if hidden > 0 {
        println!("      … 还有 {hidden} 个");
    }
    let t = query.timings;
    println!(
        "  parse {} · lookup {} · rank {} · translate {} ({}/{} hit) · total {}",
        fmt_duration(t.parse),
        fmt_duration(t.lookup),
        fmt_duration(t.rank),
        fmt_duration(report.elapsed),
        report.hits,
        report.total,
        fmt_duration(t.total() + report.elapsed),
    );
    show_prediction(engine, &query.candidates.items);
    Some(query)
}

/// 逐键模式：`kaifa` 当作 k、ka、kai…… 五次按键，每个前缀都查一次并标注译文，
/// 一行一键打印各阶段耗时和首候选。这是输入法每键的真实工作量（联想不算，它在后台线程）。
pub fn show_typing(engine: &mut Engine, input: &str) {
    println!(
        "  {:<16} {:>9} {:>9} {:>9} {:>9} {:>9}  首候选",
        "输入", "parse", "lookup", "rank", "translate", "total"
    );
    let mut worst = Duration::ZERO;
    let mut sum = Duration::ZERO;
    let mut keys = 0;
    for (index, _) in input
        .char_indices()
        .skip(1)
        .chain(std::iter::once((input.len(), ' ')))
    {
        let prefix = &input[..index];
        engine.set_input(prefix);
        let mut query = match engine.query() {
            Ok(query) => query,
            Err(error) => {
                println!("  {prefix:<16} {error}");
                continue;
            }
        };
        let report = engine.annotate(&mut query.candidates);
        let t = query.timings;
        let total = t.total() + report.elapsed;
        worst = worst.max(total);
        sum += total;
        keys += 1;
        let first = query
            .candidates
            .items
            .first()
            .map(|c| c.text.as_str())
            .unwrap_or("（无候选）");
        println!(
            "  {prefix:<16} {:>9} {:>9} {:>9} {:>9} {:>9}  {first}",
            fmt_duration(t.parse),
            fmt_duration(t.lookup),
            fmt_duration(t.rank),
            fmt_duration(report.elapsed),
            fmt_duration(total),
        );
    }
    if keys > 0 {
        println!(
            "  {keys} 键 · 最慢 {} · 平均 {}",
            fmt_duration(worst),
            fmt_duration(sum / keys)
        );
    }
    engine.set_input("");
}

/// 发一次联想并等结果打印出来。没接 Predictor 时什么都不做。
pub fn show_prediction(engine: &mut Engine, candidates: &[Candidate]) {
    let Some(_sequence) = engine.request_prediction(None, candidates) else {
        return;
    };
    let start = Instant::now();
    while start.elapsed() < PREDICTION_WAIT {
        if let Some(prediction) = engine.poll_prediction() {
            if prediction.is_empty() {
                println!("  ☁ （无联想）");
            }
            for word in &prediction.words {
                let reading = word
                    .reading
                    .clone()
                    .unwrap_or_else(|| word.syllables.join(" "));
                println!("  ☁ {}  ({reading})", word.text);
            }
            if let Some(sentence) = &prediction.sentence {
                println!("  ☁ 整句: {sentence}");
            }
            println!("  predict {}", fmt_duration(start.elapsed()));
            return;
        }
        std::thread::sleep(PREDICTION_POLL);
    }
    println!("  ☁ （联想超时）");
}

/// 候选词左对齐，右侧是「词性 译文」，多条释义用 · 分隔。
fn format_candidate(candidate: &Candidate, width: usize) -> String {
    let padding = " ".repeat(width.saturating_sub(display_width(&candidate.text)) + 2);
    let reading = candidate
        .reading
        .as_ref()
        .map(|r| format!("{r} "))
        .unwrap_or_default();
    let annotation = candidate
        .translation
        .as_ref()
        .map(|t| {
            t.senses()
                .iter()
                .map(|s| {
                    // 日文译词按汉字段注平假名：開発(かいはつ)する
                    let text: String = s
                        .furigana()
                        .iter()
                        .map(|segment| match &segment.reading {
                            Some(reading) => format!("{}({reading})", segment.text),
                            None => segment.text.clone(),
                        })
                        .collect();
                    match s.part_of_speech {
                        Some(pos) => format!("{pos} {text}"),
                        None => text,
                    }
                })
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_default();
    let marker = match candidate.kind {
        qingjian_core::CandidateKind::Chinese => "",
        qingjian_core::CandidateKind::English => "[en] ",
        qingjian_core::CandidateKind::Cloud => "☁ ",
        qingjian_core::CandidateKind::Shortcut => "[v] ",
        qingjian_core::CandidateKind::Custom(_) => "[custom] ",
        qingjian_core::CandidateKind::Sentence => "[句] ",
        qingjian_core::CandidateKind::Emoji => "",
    };
    format!("{}{padding}{marker}{reading}{annotation}", candidate.text)
        .trim_end()
        .to_owned()
}

/// 终端显示宽度：CJK 算两格。够 CLI 对齐用，不引入 unicode-width。
fn display_width(text: &str) -> usize {
    text.chars()
        .map(|c| if c as u32 >= 0x2E80 { 2 } else { 1 })
        .sum()
}

fn fmt_duration(d: Duration) -> String {
    let micros = d.as_micros();
    if micros >= 1000 {
        format!("{:.2}ms", d.as_secs_f64() * 1000.0)
    } else {
        format!("{micros}µs")
    }
}
