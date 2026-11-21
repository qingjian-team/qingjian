//! `trans` 子模式：对一串数做变换，`vtrans;log;1;2;4`。
//!
//! 第一个参数是算子名，其余是要处理的一串数（分号隔开，与 `vstat` 一样）。
//! 每条候选是「值 + 名字」：上屏的是变换后那串数，名字只画在候选旁边。
//!
//! 打乱（`shuffle`）不在这里：随机值要按组句缓存，走 `generated_forms` 那条路
//! （见 [`super::generated_forms`]），否则每敲一个键都重排一次。

use super::modes::split_args;
use super::stat::fmt;

/// 打乱那一个算子的名字（它走生成型那条路）。
pub const SHUFFLE: &str = "shuffle";

/// 一条候选：上屏的值，与只在界面上显示的名字（空串表示不带名字）。
pub type Measure = (String, String);

/// 等宽分箱分几箱。
const BINS: usize = 5;

/// 算子名归一：认几个别名，返回规范名。
pub fn canonical(op: &str) -> Option<&'static str> {
    match op.to_ascii_lowercase().as_str() {
        "minmax" | "norm" | "mm" => Some("minmax"),
        "z" | "std" | "zscore" => Some("z"),
        "log" | "ln" => Some("log"),
        "log10" | "lg" => Some("log10"),
        "sqrt" => Some("sqrt"),
        "diff" | "d" => Some("diff"),
        "smooth" | "ma" => Some("smooth"),
        "low" | "lowpass" => Some("low"),
        "high" | "highpass" => Some("high"),
        "median" | "med" => Some("median"),
        "bin" | "bin5" => Some("bin"),
        "uniq" => Some("uniq"),
        "shuffle" | "shuf" => Some(SHUFFLE),
        _ => None,
    }
}

/// `算子名;数;数;…`。算子认不出来、或有一个数认不出来时整条不出候选。
pub fn forms(args: &str) -> Vec<Measure> {
    let texts = split_args(args);
    let Some((op, rest)) = texts.split_first() else {
        return Vec::new();
    };
    // 不写算子（`vtrans;2;3;4;5;3`）就全给：把能算的都算一遍，与 `vstat;` 一个意思
    let Some(op) = canonical(op) else {
        let mut numbers = Vec::with_capacity(texts.len());
        for text in &texts {
            match text.parse() {
                Ok(value) => numbers.push(value),
                Err(_) => return Vec::new(),
            }
        }
        if numbers.is_empty() {
            return Vec::new();
        }
        let mut forms: Vec<Measure> = Vec::new();
        for op in ["minmax", "z", "diff", "smooth", "median", "bin"] {
            if let Some((values, label)) = apply(op, &numbers) {
                forms.push((join(&values), label.to_owned()));
            }
        }
        // 对数与开方只在取值范围内有意义：负数、零不上，别给 NaN
        for op in ["log", "log10", "sqrt"] {
            if let Some((values, label)) = apply(op, &numbers) {
                forms.push((join(&values), label.to_owned()));
            }
        }
        return forms;
    };
    // 打乱走生成型那条路，不走这里
    if op == SHUFFLE {
        return Vec::new();
    }
    let mut numbers = Vec::with_capacity(rest.len());
    for text in rest {
        match text.parse() {
            Ok(value) => numbers.push(value),
            Err(_) => return Vec::new(),
        }
    }
    if numbers.is_empty() {
        return Vec::new();
    }
    match apply(op, &numbers) {
        Some((values, label)) => vec![(join(&values), label.to_owned())],
        None => Vec::new(),
    }
}

/// 打乱：同一段输入只算一次（由 Engine 缓存），换一组输入才重排。
pub fn shuffle_forms(args: &str) -> Vec<String> {
    let texts = split_args(args);
    let Some((op, rest)) = texts.split_first() else {
        return Vec::new();
    };
    if canonical(op) != Some(SHUFFLE) {
        return Vec::new();
    }
    let mut numbers = Vec::with_capacity(rest.len());
    for text in rest {
        match text.parse() {
            Ok(value) => numbers.push(value),
            Err(_) => return Vec::new(),
        }
    }
    if numbers.is_empty() {
        return Vec::new();
    }
    shuffle(&mut numbers);
    vec![join(&numbers)]
}

/// 算一个算子；算不出来（有负数取对数、样本太少）时 `None`。
fn apply(op: &str, numbers: &[f64]) -> Option<(Vec<f64>, &'static str)> {
    let count = numbers.len();
    match op {
        "minmax" => {
            let low = numbers.iter().cloned().fold(f64::INFINITY, f64::min);
            let high = numbers.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let span = high - low;
            let values = numbers
                .iter()
                .map(|x| if span == 0.0 { 0.0 } else { (x - low) / span })
                .collect();
            Some((values, "最小最大归一化"))
        }
        "z" => {
            let count = count as f64;
            let mean = numbers.iter().sum::<f64>() / count;
            let deviation =
                (numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / count).sqrt();
            let values = numbers
                .iter()
                .map(|x| {
                    if deviation == 0.0 {
                        0.0
                    } else {
                        (x - mean) / deviation
                    }
                })
                .collect();
            Some((values, "标准化（z 分数）"))
        }
        "log" => {
            if numbers.iter().any(|x| *x <= 0.0) {
                return None;
            }
            Some((numbers.iter().map(|x| x.ln()).collect(), "自然对数"))
        }
        "log10" => {
            if numbers.iter().any(|x| *x <= 0.0) {
                return None;
            }
            Some((numbers.iter().map(|x| x.log10()).collect(), "常用对数"))
        }
        "sqrt" => {
            if numbers.iter().any(|x| *x < 0.0) {
                return None;
            }
            Some((numbers.iter().map(|x| x.sqrt()).collect(), "平方根"))
        }
        "diff" => {
            if count < 2 {
                return None;
            }
            Some((
                numbers.windows(2).map(|pair| pair[1] - pair[0]).collect(),
                "一阶差分",
            ))
        }
        "smooth" => Some((moving(numbers, 3, mean_of), "三点滑动平均")),
        "median" => Some((moving(numbers, 3, median_of_window), "中值滤波（窗口 3）")),
        "low" => Some((low_pass(numbers), "低通（指数平滑 α=0.5）")),
        "high" => {
            let low = low_pass(numbers);
            let values = numbers.iter().zip(low.iter()).map(|(x, y)| x - y).collect();
            Some((values, "高通（原值 − 低通）"))
        }
        "bin" => Some((bins(numbers), "等宽分箱（5 箱）")),
        "uniq" => Some((unique(numbers), "去重")),
        _ => None,
    }
}

/// 让 `fmt` 能把一串数拼成候选文本。
fn join(values: &[f64]) -> String {
    values.iter().map(|v| fmt(*v)).collect::<Vec<_>>().join(" ")
}

/// 窗口滑动：边缘处窗口缩小到还在范围内的那几个。
fn moving(numbers: &[f64], width: usize, combine: fn(&[f64]) -> f64) -> Vec<f64> {
    let half = width / 2;
    (0..numbers.len())
        .map(|i| {
            let start = i.saturating_sub(half);
            let end = (i + half + 1).min(numbers.len());
            combine(&numbers[start..end])
        })
        .collect()
}

fn mean_of(window: &[f64]) -> f64 {
    window.iter().sum::<f64>() / window.len() as f64
}

fn median_of_window(window: &[f64]) -> f64 {
    let mut sorted = window.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[middle]
    } else {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    }
}

/// 一阶指数平滑：`y0 = x0`，之后 `y = αx + (1−α)y₋₁`，α 取 0.5。
fn low_pass(numbers: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(numbers.len());
    let mut previous = numbers[0];
    out.push(previous);
    for value in &numbers[1..] {
        previous = 0.5 * value + 0.5 * previous;
        out.push(previous);
    }
    out
}

/// 等宽分箱：给每个数落在第几箱（从 1 数）。全部相等时都在第一箱。
fn bins(numbers: &[f64]) -> Vec<f64> {
    let low = numbers.iter().cloned().fold(f64::INFINITY, f64::min);
    let high = numbers.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = high - low;
    numbers
        .iter()
        .map(|x| {
            if span == 0.0 {
                1.0
            } else {
                let index = ((x - low) / span * BINS as f64).floor();
                index.clamp(0.0, (BINS - 1) as f64) + 1.0
            }
        })
        .collect()
}

/// 去重，保持先后顺序（同一个数只留第一次出现的）。
fn unique(numbers: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::with_capacity(numbers.len());
    for value in numbers {
        if !out.iter().any(|seen| (seen - value).abs() < 1e-9) {
            out.push(*value);
        }
    }
    out
}

/// Fisher–Yates。随机源用 `uuid` 的 v4（已经是要用的依赖），不必为此加一个 rand。
fn shuffle(numbers: &mut [f64]) {
    let mut state = seed();
    for i in (1..numbers.len()).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = ((state >> 33) as usize) % (i + 1);
        numbers.swap(i, j);
    }
}

fn seed() -> u64 {
    let bytes = uuid::Uuid::new_v4().into_bytes();
    let mut seed = 0u64;
    for chunk in bytes.chunks_exact(8) {
        seed ^= u64::from_le_bytes(chunk.try_into().unwrap_or_default());
    }
    seed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(args: &str) -> Option<String> {
        forms(args).into_iter().next().map(|(value, _)| value)
    }

    fn label(args: &str) -> Option<String> {
        forms(args).into_iter().next().map(|(_, label)| label)
    }

    #[test]
    fn min_max_and_z_score() {
        assert_eq!(value("minmax;1;2;3;5").as_deref(), Some("0 0.25 0.5 1"));
        assert_eq!(label("minmax;1;2;3;5").as_deref(), Some("最小最大归一化"));
        // 全相等时不给 NaN
        assert_eq!(value("minmax;4;4").as_deref(), Some("0 0"));
        // 标准化：均 0、方差 1
        let z = value("z;1;2;3").unwrap_or_default();
        assert!(z.starts_with("-1.224745 0 1.224745"), "{z}");
        assert_eq!(value("z;5;5").as_deref(), Some("0 0"));
    }

    #[test]
    fn logs_and_roots_need_the_right_range() {
        assert_eq!(value("log;1;2").as_deref(), Some("0 0.693147"));
        assert_eq!(value("log10;1;100").as_deref(), Some("0 2"));
        assert!(value("log;0;2").is_none(), "零没有对数");
        assert!(value("log;-1;2").is_none());
        assert_eq!(value("sqrt;0;4;9").as_deref(), Some("0 2 3"));
        assert!(value("sqrt;-1;4").is_none());
    }

    #[test]
    fn diff_needs_two_numbers() {
        assert_eq!(value("diff;1;4;9").as_deref(), Some("3 5"));
        assert!(value("diff;7").is_none());
    }

    #[test]
    fn smoothing_low_and_high_pass() {
        // 三点滑动平均：边缘窗口缩小
        assert_eq!(value("smooth;1;2;6").as_deref(), Some("1.5 3 4"));
        // 中值滤波：边缘窗口缩到两个数，取两者均值（1 100 → 50.5、100 2 → 51）
        assert_eq!(value("median;1;100;2").as_deref(), Some("50.5 2 51"));
        // 低通：y0 = x0，之后 α=0.5
        assert_eq!(value("low;0;10;0").as_deref(), Some("0 5 2.5"));
        assert_eq!(value("high;0;10;0").as_deref(), Some("0 5 -2.5"));
    }

    #[test]
    fn bins_and_unique() {
        assert_eq!(value("bin;0;1;2;3;4").as_deref(), Some("1 2 3 4 5"));
        assert_eq!(value("bin;5;5;5").as_deref(), Some("1 1 1"));
        assert_eq!(value("uniq;1;2;1;3;2").as_deref(), Some("1 2 3"));
    }

    #[test]
    fn unknown_op_or_bad_number_gives_nothing() {
        assert!(forms("nope;1;2").is_empty());
        assert!(forms("log;1;x").is_empty(), "`x` 不该被丢掉");
        assert!(forms("log").is_empty());
        assert!(forms("").is_empty());
    }

    #[test]
    fn shuffle_keeps_every_number_and_needs_the_op() {
        let shuffled = shuffle_forms("shuffle;1;2;3;4;5");
        let text = shuffled.first().cloned().unwrap_or_default();
        let mut seen: Vec<&str> = text.split(' ').collect();
        seen.sort_unstable();
        assert_eq!(seen, ["1", "2", "3", "4", "5"], "{text}");
        assert!(shuffle_forms("log;1;2").is_empty());
    }
}
