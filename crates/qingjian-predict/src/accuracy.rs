//! 校验模型不能擅自增减数字，术语必须遵循用户显式指定的译法。

pub(crate) fn valid(source: &str, translated: &str, language: &str, terms: &[String]) -> bool {
    if translated.trim().is_empty() || !numbers_valid(source, translated) {
        return false;
    }
    for term in terms {
        if let Some((original, pair)) = term.split_once('=')
            && !original.is_empty()
            && source.contains(original)
            && let Some((en, ja)) = pair.split_once('|')
        {
            let target = match language {
                "en" => en,
                "ja" => ja,
                _ => continue,
            }
            .trim();
            if !target.is_empty() && !translated.to_lowercase().contains(&target.to_lowercase()) {
                return false;
            }
        }
    }
    true
}

fn numbers(text: &str) -> Vec<String> {
    let normalized: String = text.chars().filter(|c| *c != ',').collect();
    let mut result: Vec<String> = normalized
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect();
    result.sort();
    result
}

fn numbers_valid(source: &str, target: &str) -> bool {
    let mut actual = numbers(target);
    for number in numbers(source) {
        let Some(index) = actual.iter().position(|n| n == &number) else {
            return false;
        };
        actual.remove(index);
    }
    let mut optional = chinese_numbers(source);
    for number in actual {
        let Some(index) = optional.iter().position(|n| n == &number) else {
            return false;
        };
        optional.remove(index);
    }
    true
}

fn chinese_numbers(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut part = String::new();
    for ch in text.chars().chain(std::iter::once(' ')) {
        if "零〇一二三四五六七八九十百千两".contains(ch) {
            part.push(ch);
            continue;
        }
        if !part.is_empty() && "分天点人次个小元日年月秒米岁条份公".contains(ch) {
            let mut total = 0u64;
            let mut digit = 0u64;
            let mut positional = 0u64;
            let mut units = false;
            for c in part.chars() {
                let d = match c {
                    '零' | '〇' => Some(0),
                    '一' => Some(1),
                    '二' | '两' => Some(2),
                    '三' => Some(3),
                    '四' => Some(4),
                    '五' => Some(5),
                    '六' => Some(6),
                    '七' => Some(7),
                    '八' => Some(8),
                    '九' => Some(9),
                    _ => None,
                };
                if let Some(d) = d {
                    digit = d;
                    positional = positional.saturating_mul(10).saturating_add(d);
                } else {
                    units = true;
                    let unit = match c {
                        '十' => 10,
                        '百' => 100,
                        '千' => 1000,
                        _ => 1,
                    };
                    total += digit.max(1) * unit;
                    digit = 0;
                }
            }
            result.push(if units { total + digit } else { positional }.to_string());
        }
        part.clear();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_changed_and_invented_numbers() {
        assert!(valid(
            "10点来，价格1,200元",
            "Come at 10, price 1200 yuan",
            "en",
            &[]
        ));
        assert!(!valid("10点", "11 o'clock", "en", &[]));
        assert!(!valid("明天来", "Come in 2 days", "en", &[]));
        assert!(valid("十分钟后", "In 10 minutes", "en", &[]));
        assert!(!valid("十分钟后", "In 11 minutes", "en", &[]));
        assert!(valid("二〇二六年", "2026", "en", &[]));
    }
    #[test]
    fn enforces_explicit_target_terminology() {
        let terms = vec!["青简=Qingjian|青簡".to_owned()];
        assert!(valid("青简很好", "Qingjian is good", "en", &terms));
        assert!(!valid("青简很好", "The software is good", "en", &terms));
        assert!(valid("青简很好", "青簡は良いです", "ja", &terms));
    }
}
