//! `stat` 子模式：一串数的统计量，`vstat;1;2;3;4`。
//!
//! 给平均、中位数、众数、方差（总体与样本各一档）、标准差、极差、总和与排序结果。
//! 参数用分号隔开（全角也认）。逗号不行 —— 中文模式下它是标点、会直接上屏。
//!
//! 每条候选是「值 + 名字」：上屏的只有值，名字由壳画在候选旁边（`Candidate::reading`），
//! 光一串数字分不出哪个是哪个。常用的几种排前面，纯值的那一份垫在后面。

use super::modes::split_args;

/// 判定两个浮点是不是「同一个数」（众数计数用）。
const EPSILON: f64 = 1e-9;

/// 一条候选：上屏的值，与只在界面上显示的名字（空串表示不带名字）。
pub type Measure = (String, String);

/// `stat;<数>;<数>;…`。
pub fn measures(args: &str) -> Vec<Measure> {
    // 开头可以点名要哪一项：`vstat;avg;3;4;2;2` 只给平均数。不点就全给。
    let (wanted, args) = pick_named(args);
    // 有一个认不出来的就整条不出候选：`vstat;1;2;x` 悄悄按 1、2 算完、看着像是对的，
    // 用户不会发现那个 `x` 被吃了。与 `vbase` 参数不对时的处理一致。
    let texts = split_args(args);
    let mut numbers: Vec<f64> = Vec::with_capacity(texts.len());
    for text in &texts {
        match text.parse() {
            Ok(value) => numbers.push(value),
            Err(_) => return Vec::new(),
        }
    }
    if numbers.is_empty() {
        return Vec::new();
    }

    let count = numbers.len();
    let sum: f64 = numbers.iter().sum();
    let mean = sum / count as f64;

    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = median_of(&numbers);
    let quartiles = quartiles_of(&numbers);

    // 总体方差（除 n）与样本方差（除 n−1）都给：前者是这套数据的离散度，后者是估计量
    let squares: f64 = numbers.iter().map(|x| (x - mean).powi(2)).sum();
    let variance = squares / count as f64;
    let sample_variance = (count > 1).then(|| squares / (count - 1) as f64);
    let deviation = variance.sqrt();
    let (low, high) = (numbers[0], numbers[count - 1]);
    let ordered: Vec<String> = numbers.iter().map(|v| fmt(*v)).collect();
    let modes_text = modes(&numbers).map(|modes| {
        let joined: Vec<String> = modes.iter().map(|v| fmt(*v)).collect();
        joined.join("、")
    });

    // 常用的几种排前面，按「先看什么」的顺序：平均、中位、众数、个数、总和。
    let mut forms: Vec<Measure> = vec![
        (fmt(mean), "平均数".to_owned()),
        (fmt(median), "中位数".to_owned()),
    ];
    if let Some(modes) = &modes_text {
        forms.push((modes.clone(), "众数".to_owned()));
    }
    if let Some((q1, q3)) = quartiles {
        forms.push((fmt(q1), "第一四分位数".to_owned()));
        forms.push((fmt(q3), "第三四分位数".to_owned()));
        forms.push((fmt(q3 - q1), "四分位距".to_owned()));
    }
    forms.extend([
        (count.to_string(), "个数".to_owned()),
        (fmt(sum), "总和".to_owned()),
        (fmt(high - low), "极差".to_owned()),
        (fmt(low), "最小".to_owned()),
        (fmt(high), "最大".to_owned()),
        (fmt(variance), "总体方差".to_owned()),
    ]);
    if let Some(sample) = sample_variance {
        forms.push((fmt(sample), "样本方差".to_owned()));
    }
    forms.push((fmt(deviation), "标准差".to_owned()));
    if let Some(sample) = sample_variance {
        forms.push((fmt(sample.sqrt()), "标准差（样本）".to_owned()));
    }
    // 全是正数才有几何平均（要取对数）
    if numbers.iter().all(|x| *x > 0.0) {
        let geometric = (numbers.iter().map(|x| x.ln()).sum::<f64>() / count as f64).exp();
        forms.push((fmt(geometric), "几何平均".to_owned()));
    }
    forms.push((ordered.join("　"), "排序".to_owned()));

    if let Some(wanted) = wanted {
        forms.retain(|(_, label)| label.starts_with(wanted));
    }
    forms
}

/// 开头那个词是不是统计项的名字；是就返回它对应的标签，并把这段从参数里摘掉。
///
/// 别名按习惯多收几个：`q1` / `m25` / `p25` / `lower` 都是第一四分位，`m50` 是中位数。
fn pick_named(args: &str) -> (Option<&'static str>, &str) {
    let text = args.trim_start();
    let Some((head, rest)) = text.split_once([';', '；', ',', '，', ' ']) else {
        return (None, args);
    };
    match label_of(head.trim()) {
        Some(label) => (Some(label), rest),
        None => (None, args),
    }
}

/// 统计项的名字与别名，全小写比。
fn label_of(name: &str) -> Option<&'static str> {
    Some(match name.to_ascii_lowercase().as_str() {
        "avg" | "mean" | "average" | "平均" | "平均数" => "平均数",
        "median" | "mid" | "m50" | "p50" | "中位数" => "中位数",
        "mode" | "众数" => "众数",
        "q1" | "m25" | "p25" | "lower" | "第一四分位" => "第一四分位数",
        "q3" | "m75" | "p75" | "upper" | "第三四分位" => "第三四分位数",
        "iqr" | "四分位距" => "四分位距",
        "count" | "n" | "个数" => "个数",
        "sum" | "total" | "总和" => "总和",
        "range" | "极差" => "极差",
        "min" | "最小" => "最小",
        "max" | "最大" => "最大",
        "var" | "variance" | "总体方差" => "总体方差",
        "svar" | "样本方差" => "样本方差",
        "sd" | "std" | "stddev" | "标准差" => "标准差",
        "geo" | "geomean" | "几何平均" => "几何平均",
        "sort" | "sorted" | "排序" => "排序",
        _ => return None,
    })
}

/// 已经排好序的一串数的中位数。
fn median_of(sorted: &[f64]) -> f64 {
    let count = sorted.len();
    if count % 2 == 1 {
        sorted[count / 2]
    } else {
        (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
    }
}

/// 第一、第三四分位数：下半个与上半个各自的中位数（奇数个时不含正中那个）。
///
/// 四个数以下不给：那时下半个只剩一个数，算出来就是最小值，没什么可看的。
fn quartiles_of(sorted: &[f64]) -> Option<(f64, f64)> {
    let count = sorted.len();
    if count < 4 {
        return None;
    }
    let half = count / 2;
    Some((
        median_of(&sorted[..half]),
        median_of(&sorted[count - half..]),
    ))
}

/// 出现次数最多的值（并列时都给）；每个数只出现一次时没有众数。
fn modes(numbers: &[f64]) -> Option<Vec<f64>> {
    let mut counts: Vec<(f64, usize)> = Vec::new();
    for &value in numbers {
        match counts
            .iter_mut()
            .find(|(seen, _)| (value - *seen).abs() < EPSILON)
        {
            Some((_, hits)) => *hits += 1,
            None => counts.push((value, 1)),
        }
    }
    let top = counts.iter().map(|(_, hits)| *hits).max()?;
    if top < 2 {
        return None;
    }
    let mut picked: Vec<f64> = counts
        .into_iter()
        .filter(|(_, hits)| *hits == top)
        .map(|(value, _)| value)
        .collect();
    picked.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(picked)
}

/// 浮点写成给人看的短串：整数不带小数点，小数最多留 6 位且不留尾随零。
pub(super) fn fmt(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let text = format!("{value:.6}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(args: &str) -> Vec<String> {
        measures(args).into_iter().map(|(value, _)| value).collect()
    }

    fn labels(args: &str) -> Vec<String> {
        measures(args)
            .into_iter()
            .map(|(_, label)| label)
            .filter(|label| !label.is_empty())
            .collect()
    }

    fn named(args: &str, name: &str) -> Option<String> {
        measures(args)
            .into_iter()
            .find(|(_, label)| label == name)
            .map(|(value, _)| value)
    }

    /// 常用的几种排在前面，名字都在；上屏的值只是那个数。
    #[test]
    fn useful_measures_come_first_with_their_names() {
        let measured = measures("1;2;3;4");
        assert_eq!(measured[0], ("2.5".to_owned(), "平均数".to_owned()));
        assert_eq!(measured[1], ("2.5".to_owned(), "中位数".to_owned()));
        assert_eq!(named("1;2;3;4", "个数").as_deref(), Some("4"));
        assert_eq!(named("1;2;3;4", "总和").as_deref(), Some("10"));
        assert_eq!(named("1;2;3;4", "极差").as_deref(), Some("3"));
        assert_eq!(named("1;2;3;4", "最小").as_deref(), Some("1"));
        assert_eq!(named("1;2;3;4", "最大").as_deref(), Some("4"));
        assert!(named("1;2;3;4", "总体方差").is_some());
        assert!(named("1;2;3;4", "标准差").is_some());
        assert!(named("1;2;3;4", "排序").is_some());
    }

    /// 四个数以上才给四分位数：1 2 3 4 的下半个是 1 2（中位 1.5）、上半个是 3 4（中位 3.5）。
    #[test]
    fn quartiles_need_four_numbers() {
        assert_eq!(named("1;2;3;4", "第一四分位数").as_deref(), Some("1.5"));
        assert_eq!(named("1;2;3;4", "第三四分位数").as_deref(), Some("3.5"));
        assert_eq!(named("1;2;3;4", "四分位距").as_deref(), Some("2"));
        // 五个数：不含正中的 3，下半个 1 2、上半个 4 5
        assert_eq!(named("1;2;3;4;5", "第一四分位数").as_deref(), Some("1.5"));
        assert_eq!(named("1;2;3;4;5", "第三四分位数").as_deref(), Some("4.5"));
        assert!(named("1;2;3", "第一四分位数").is_none(), "三个数不给");
    }

    #[test]
    fn odd_count_median_is_middle() {
        assert_eq!(measures("1;2;9")[1], ("2".to_owned(), "中位数".to_owned()));
    }

    #[test]
    fn mode_reports_ties() {
        let tied = measures("1;1;2;2;3");
        assert!(
            tied.iter().any(|(v, l)| v == "1、2" && l == "众数"),
            "{tied:?}"
        );
        // 每个数只出现一次时不给众数
        let distinct = measures("1;2;3");
        assert!(!distinct.iter().any(|(_, l)| l == "众数"), "{distinct:?}");
    }

    #[test]
    fn sample_variance_only_with_two_or_more() {
        let one = labels("5");
        assert!(!one.contains(&"样本方差".to_owned()), "{one:?}");
        let two = values("1;3");
        assert!(two.contains(&"2".to_owned()), "{two:?}");
    }

    /// 全是正数才给几何平均。
    #[test]
    fn geometric_mean_needs_positive_numbers() {
        assert!(labels("1;4").contains(&"几何平均".to_owned()));
        assert!(!labels("-1;4").contains(&"几何平均".to_owned()));
    }

    #[test]
    fn full_width_semicolon_is_accepted() {
        assert_eq!(values("1；2；3；4")[0], "2.5");
    }

    /// 一个参数认不出来就整条不出候选，不悄悄按剩下的算。
    #[test]
    fn one_bad_argument_kills_the_whole_set() {
        assert!(measures("").is_empty());
        assert!(measures("abc;def").is_empty());
        assert!(measures("1;2;x").is_empty(), "`x` 不该被丢掉");
        assert!(measures("1;2;3.5x").is_empty());
    }
}
