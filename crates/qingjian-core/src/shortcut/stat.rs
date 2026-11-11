//! `stat` 子模式：一串数的统计量，`vstat;1;2;3;4`。
//!
//! 给平均、中位数、众数、方差（总体与样本各一档）、标准差、极差、总和与排序结果。
//! 参数用分号隔开（全角也认）。逗号不行 —— 中文模式下它是标点、会直接上屏。

use super::modes::split_args;

/// 判定两个浮点是不是「同一个数」（众数计数用）。
const EPSILON: f64 = 1e-9;

/// `stat;<数>;<数>;…`。
pub fn forms(args: &str) -> Vec<String> {
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

    let mut forms = vec![fmt(mean), fmt(median)];
    if let Some(modes) = modes(&numbers) {
        let joined: Vec<String> = modes.iter().map(|v| fmt(*v)).collect();
        forms.push(joined.join("、"));
    }
    forms.push(fmt(variance));
    if let Some(sample) = sample_variance {
        forms.push(fmt(sample));
    }
    forms.push(fmt(variance.sqrt()));
    let (low, high) = (numbers[0], numbers[count - 1]);
    forms.push(fmt(high - low));
    forms.push(format!("{}　{}", fmt(low), fmt(high)));
    forms.push(fmt(sum));
    forms.push(count.to_string());
    let ordered: Vec<String> = numbers.iter().map(|v| fmt(*v)).collect();
    forms.push(ordered.join(" "));
    forms
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
fn fmt(value: f64) -> String {
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

    /// 候选项是纯数字：选一条上屏的就是那个数，不带「平均」这类名字。
    /// 顺序固定（平均、中位数、众数、方差、样本方差、标准差、极差、最小最大、总和、个数、排序），
    /// 用户按位置挑。
    #[test]
    fn basic_measures() {
        let measured = forms("1;2;3;4");
        assert_eq!(measured[0], "2.5", "{measured:?}");
        assert_eq!(measured[1], "2.5", "{measured:?}");
        assert_eq!(measured[2], "1.25", "{measured:?}");
        assert_eq!(measured.last().map(String::as_str), Some("1 2 3 4"));
        assert!(measured.iter().any(|f| f == "10"), "总和：{measured:?}");
        assert!(measured.iter().any(|f| f == "4"), "个数：{measured:?}");
    }

    #[test]
    fn odd_count_median_is_middle() {
        assert_eq!(forms("1;2;9")[1], "2");
    }

    #[test]
    fn mode_reports_ties() {
        let tied = forms("1;1;2;2;3");
        assert!(tied.iter().any(|f| f == "1、2"), "{tied:?}");
        // 每个数只出现一次时不给众数：第三条直接是方差
        assert_eq!(forms("1;2;3")[2], "0.666667");
    }

    #[test]
    fn sample_variance_only_with_two_or_more() {
        // 一个数没有样本方差（要除以 n−1，会除零）：平均、中位数、方差、标准差、极差…
        let one = forms("5");
        assert_eq!(one[0], "5");
        // 两个数就有，第四条
        let two = forms("1;3");
        assert_eq!(two[3], "2", "{two:?}");
    }

    #[test]
    fn full_width_semicolon_is_accepted() {
        assert_eq!(forms("1；2；3；4")[0], "2.5");
    }

    /// 一个参数认不出来就整条不出候选，不悄悄按剩下的算。
    #[test]
    fn one_bad_argument_kills_the_whole_set() {
        assert!(forms("").is_empty());
        assert!(forms("abc;def").is_empty());
        assert!(forms("1;2;x").is_empty(), "`x` 不该被丢掉");
        assert!(forms("1;2;3.5x").is_empty());
    }
}
