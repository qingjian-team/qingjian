//! `stat` 子模式：一串数的统计量，`vstat;1;2;3;4`。
//!
//! 给平均、中位数、四分位、众数、方差（总体与样本各一档）、标准差、极差、总和与排序结果。
//! 参数用分号隔开（全角也认）。逗号不行 —— 中文模式下它是标点、会直接上屏。
//!
//! 开头可以点名要哪一项：`vstat;avg;1;2;3` 只给平均数、`vstat;q1;1;2;3;4` 第一四分位数，
//! 别名见 `label_of`。不点名就全给，常用的排前面。

use super::modes::split_args;

/// 判定两个浮点是不是「同一个数」（众数计数用）。
const EPSILON: f64 = 1e-9;

/// `stat;<数>;<数>;…`。
pub fn forms(args: &str) -> Vec<(String, String)> {
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
    let median = if count % 2 == 1 {
        numbers[count / 2]
    } else {
        (numbers[count / 2 - 1] + numbers[count / 2]) / 2.0
    };

    // 总体方差（除 n）与样本方差（除 n−1）都给：前者是这套数据的离散度，后者是估计量
    let squares: f64 = numbers.iter().map(|x| (x - mean).powi(2)).sum();
    let variance = squares / count as f64;
    let sample_variance = (count > 1).then(|| squares / (count - 1) as f64);

    // 常用的排前面：平均、中位、四分位，然后众数与各档离散度。标签只在内部用来点名过滤。
    let mut items: Vec<(String, &'static str)> =
        vec![(fmt(mean), "平均数"), (fmt(median), "中位数")];
    if let Some((q1, q3)) = quartiles_of(&numbers) {
        items.push((fmt(q1), "第一四分位数"));
        items.push((fmt(q3), "第三四分位数"));
        items.push((fmt(q3 - q1), "四分位距"));
    }
    if let Some(modes) = modes(&numbers) {
        let joined: Vec<String> = modes.iter().map(|v| fmt(*v)).collect();
        items.push((joined.join("、"), "众数"));
    }
    items.push((fmt(variance), "总体方差"));
    if let Some(sample) = sample_variance {
        items.push((fmt(sample), "样本方差"));
    }
    items.push((fmt(variance.sqrt()), "标准差"));
    let (low, high) = (numbers[0], numbers[count - 1]);
    items.push((fmt(high - low), "极差"));
    items.push((fmt(low), "最小"));
    items.push((fmt(high), "最大"));
    items.push((fmt(sum), "总和"));
    items.push((count.to_string(), "个数"));
    let ordered: Vec<String> = numbers.iter().map(|v| fmt(*v)).collect();
    items.push((ordered.join(" "), "排序"));

    items
        .into_iter()
        .filter(|(_, label)| wanted.is_none_or(|wanted| *label == wanted))
        .map(|(value, label)| (value, label.to_owned()))
        .collect()
}

/// 开头那个词是不是统计项的名字；是就返回它对应的标签，并把这段从参数里摘掉。
///
/// 别名按习惯多收几个：`q1` / `m25` / `p25` / `lower` 都是第一四分位数，`m50` 是中位数。
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
        "q1" | "m25" | "p25" | "lower" | "第一四分位" => "第一四分位数",
        "q3" | "m75" | "p75" | "upper" | "第三四分位" => "第三四分位数",
        "iqr" | "四分位距" => "四分位距",
        "mode" | "众数" => "众数",
        "var" | "variance" | "总体方差" => "总体方差",
        "svar" | "样本方差" => "样本方差",
        "sd" | "std" | "stddev" | "标准差" => "标准差",
        "range" | "极差" => "极差",
        "min" | "最小" => "最小",
        "max" | "最大" => "最大",
        "sum" | "total" | "总和" => "总和",
        "count" | "n" | "个数" => "个数",
        "sort" | "sorted" | "排序" => "排序",
        _ => return None,
    })
}

/// 四分位数：把已排好的数一分为二，各自取中位数。少于四个数谈不上四分位。
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

/// 一段已排好的数的中位数。
fn median_of(sorted: &[f64]) -> f64 {
    let count = sorted.len();
    if count % 2 == 1 {
        sorted[count / 2]
    } else {
        (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
    }
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
pub(crate) fn fmt(value: f64) -> String {
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

    /// 用例只关心值：说明（「平均数」这些）另有用例看着。
    fn values(args: &str) -> Vec<String> {
        forms(args).into_iter().map(|(value, _)| value).collect()
    }

    /// 候选项是纯数字：选一条上屏的就是那个数，不带「平均」这类名字。
    /// 顺序固定（平均、中位数、四分位三档、众数、方差…、总和、个数、排序），用户按位置挑。
    #[test]
    fn basic_measures() {
        let measured = values("1;2;3;4");
        assert_eq!(measured[0], "2.5", "{measured:?}");
        assert_eq!(measured[1], "2.5", "{measured:?}");
        assert_eq!(measured[2], "1.5", "第一四分位数：{measured:?}");
        assert_eq!(measured[3], "3.5", "第三四分位数：{measured:?}");
        assert_eq!(measured[4], "2", "四分位距：{measured:?}");
        assert_eq!(measured.last().map(String::as_str), Some("1 2 3 4"));
        assert!(measured.iter().any(|f| f == "10"), "总和：{measured:?}");
        assert!(measured.iter().any(|f| f == "4"), "个数：{measured:?}");
    }

    /// 开头点一项就只给这一项，别名也认。
    #[test]
    fn named_measures_only_give_that_one() {
        assert_eq!(values("avg;1;2;3"), ["2"]);
        assert_eq!(values("q1;1;2;3;4"), ["1.5"]);
        assert_eq!(values("m75;1;2;3;4"), ["3.5"], "`m75` 就是第三四分位");
        assert_eq!(values("iqr;1;2;3;4"), ["2"]);
        assert_eq!(values("中位数;1;2;9"), ["2"], "中文名也认");
        assert_eq!(values("sort;3;1;2"), ["1 2 3"]);
        // 点名之后剩的参数照常解析；一个都认不出就没有候选
        assert!(values("avg;1;x").is_empty());
        assert!(values("没有这项;1;2").is_empty() || values("没有这项;1;2")[0] != "2");
    }

    /// 少于四个数没有四分位，也不会因此少别的项。
    #[test]
    fn quartiles_need_four_numbers() {
        let three = values("1;2;3");
        assert!(
            !three.iter().any(|f| f == "1.5"),
            "三个数没有四分位：{three:?}"
        );
        assert_eq!(three[0], "2", "{three:?}");
        // 四个数就有了
        let four = values("1;2;3;100");
        assert!(four.iter().any(|f| f == "1.5"), "第一四分位：{four:?}");
        assert!(four.iter().any(|f| f == "51.5"), "第三四分位：{four:?}");
    }

    #[test]
    fn odd_count_median_is_middle() {
        assert_eq!(values("1;2;9")[1], "2");
    }

    #[test]
    fn mode_reports_ties() {
        let tied = values("1;1;2;2;3");
        assert!(tied.iter().any(|f| f == "1、2"), "{tied:?}");
        // 每个数只出现一次时不给众数：第三条直接是方差
        assert_eq!(values("1;2;3")[2], "0.666667");
    }

    #[test]
    fn sample_variance_only_with_two_or_more() {
        // 一个数没有样本方差（要除以 n−1，会除零）：平均、中位数、方差、标准差、极差…
        let one = values("5");
        assert_eq!(one[0], "5");
        // 两个数就有，第四条
        let two = values("1;3");
        assert_eq!(two[3], "2", "{two:?}");
    }

    #[test]
    fn full_width_semicolon_is_accepted() {
        assert_eq!(values("1；2；3；4")[0], "2.5");
    }

    /// 一个参数认不出来就整条不出候选，不悄悄按剩下的算。
    #[test]
    fn one_bad_argument_kills_the_whole_set() {
        assert!(values("").is_empty());
        assert!(values("abc;def").is_empty());
        assert!(values("1;2;x").is_empty(), "`x` 不该被丢掉");
        assert!(values("1;2;3.5x").is_empty());
    }
}
