//! `stat` 子模式：一串数的统计量，`vstat;1;2;3;4`。
//!
//! 给平均、中位数、众数、方差（总体与样本各一档）、标准差、极差、总和与排序结果。
//! 参数用分号（或逗号）隔开，与其它带参数的子模式一致。

use super::modes::split_args;

/// 判定两个浮点是不是「同一个数」（众数计数用）。
const EPSILON: f64 = 1e-9;

/// `stat;<数>;<数>;…`。
pub fn forms(args: &str) -> Vec<String> {
    let mut numbers: Vec<f64> = split_args(args)
        .into_iter()
        .filter_map(|text| text.parse().ok())
        .collect();
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

    let mut forms = vec![
        format!("平均 {}", fmt(mean)),
        format!("中位数 {}", fmt(median)),
    ];
    if let Some(modes) = modes(&numbers) {
        let joined: Vec<String> = modes.iter().map(|v| fmt(*v)).collect();
        forms.push(format!("众数 {}", joined.join("、")));
    }
    forms.push(format!("方差 {}", fmt(variance)));
    if let Some(sample) = sample_variance {
        forms.push(format!("样本方差 {}", fmt(sample)));
    }
    forms.push(format!("标准差 {}", fmt(variance.sqrt())));
    let (low, high) = (numbers[0], numbers[count - 1]);
    forms.push(format!("极差 {}", fmt(high - low)));
    forms.push(format!("最小 {}　最大 {}", fmt(low), fmt(high)));
    forms.push(format!("总和 {}", fmt(sum)));
    forms.push(format!("个数 {count}"));
    let ordered: Vec<String> = numbers.iter().map(|v| fmt(*v)).collect();
    forms.push(format!("排序 {}", ordered.join(" ")));
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

    #[test]
    fn basic_measures() {
        let measured = forms("1;2;3;4");
        assert!(measured.iter().any(|f| f == "平均 2.5"), "{measured:?}");
        assert!(measured.iter().any(|f| f == "中位数 2.5"), "{measured:?}");
        assert!(measured.iter().any(|f| f == "方差 1.25"), "{measured:?}");
        assert!(measured.iter().any(|f| f == "极差 3"), "{measured:?}");
        assert!(measured.iter().any(|f| f == "总和 10"), "{measured:?}");
        assert!(measured.iter().any(|f| f == "排序 1 2 3 4"), "{measured:?}");
    }

    #[test]
    fn odd_count_median_is_middle() {
        assert!(forms("1;2;9").iter().any(|f| f == "中位数 2"));
    }

    #[test]
    fn mode_reports_ties() {
        let tied = forms("1;1;2;2;3");
        assert!(tied.iter().any(|f| f == "众数 1、2"), "{tied:?}");
        // 每个数只出现一次时不给众数
        assert!(!forms("1;2;3").iter().any(|f| f.starts_with("众数")));
    }

    #[test]
    fn sample_variance_only_with_two_or_more() {
        // 一个数没有样本方差（要除以 n−1，会除零）
        assert!(!forms("5").iter().any(|f| f.starts_with("样本方差")));
        // 两个数就有
        assert!(forms("1;3").iter().any(|f| f.starts_with("样本方差")));
    }

    #[test]
    fn accepts_commas_and_full_width() {
        assert!(forms("1,2,3,4").iter().any(|f| f == "平均 2.5"));
        assert!(forms("1；2；3；4").iter().any(|f| f == "平均 2.5"));
    }

    #[test]
    fn bad_input_is_empty() {
        assert!(forms("").is_empty());
        assert!(forms("abc;def").is_empty());
    }
}
