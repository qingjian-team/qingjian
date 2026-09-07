//! 阿拉伯数字串转中文数字：小写（一百二十三）与大写（壹佰贰拾叁，财务用）。

/// 小写数字与单位。
const LOWER: Numeral = Numeral {
    digits: ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"],
    units: ["", "十", "百", "千"],
    // 口语里 10–19 开头的「一」省掉：十五，而不是一十五
    drop_leading_one_ten: true,
};

/// 大写数字与单位，财务票据写法。
const UPPER: Numeral = Numeral {
    digits: ["零", "壹", "贰", "叁", "肆", "伍", "陆", "柒", "捌", "玖"],
    units: ["", "拾", "佰", "仟"],
    drop_leading_one_ten: false,
};

/// 亿以内每四位一节的节单位；亿以上把高位部分整体读出再接「亿」（一万二千三百四十五亿）。
const SECTIONS: [&str; 2] = ["", "万"];

/// 一「亿」有几位。
const YI_DIGITS: usize = 8;

/// 一套数字写法。
struct Numeral {
    /// 0–9。
    digits: [&'static str; 10],

    /// 个、十、百、千。
    units: [&'static str; 4],

    /// 整个数以「一十」开头时是否省掉「一」。
    drop_leading_one_ten: bool,
}

/// `123` → `一百二十三`。只接受纯数字串；超过 16 位或空串原样返回。
pub fn chinese_lower(digits: &str) -> String {
    LOWER.convert(digits)
}

/// `123` → `壹佰贰拾叁`。
pub fn chinese_upper(digits: &str) -> String {
    UPPER.convert(digits)
}

impl Numeral {
    fn convert(&self, digits: &str) -> String {
        let digits = digits.trim_start_matches('0');
        if digits.is_empty() {
            return self.digits[0].to_owned();
        }
        if !digits.bytes().all(|b| b.is_ascii_digit()) || digits.len() > 16 {
            return digits.to_owned();
        }
        let text = self.below_yi_or_split(digits);
        if self.drop_leading_one_ten
            && let Some(rest) = text.strip_prefix(&format!("{}{}", self.digits[1], self.units[1]))
        {
            return format!("{}{rest}", self.units[1]);
        }
        text
    }

    /// 亿以上：高位整体转换后接「亿」，低八位非零时接上（不足八位补「零」）。
    fn below_yi_or_split(&self, digits: &str) -> String {
        if digits.len() <= YI_DIGITS {
            return self.below_yi(digits);
        }
        let (high, low) = digits.split_at(digits.len() - YI_DIGITS);
        let mut text = self.below_yi_or_split(high);
        text.push('亿');
        let low = low.trim_start_matches('0');
        if !low.is_empty() {
            if low.len() < YI_DIGITS {
                text.push_str(self.digits[0]);
            }
            text.push_str(&self.below_yi(low));
        }
        text
    }

    /// 亿以内（最多八位，无前导零）：从低位起每四位一节。
    fn below_yi(&self, digits: &str) -> String {
        let values: Vec<usize> = digits.bytes().map(|b| usize::from(b - b'0')).collect();
        let mut sections: Vec<&[usize]> = values.rchunks(4).collect();
        sections.reverse();
        let count = sections.len();
        let mut text = String::new();
        let mut pending_zero = false;
        for (index, section) in sections.iter().enumerate() {
            let value: usize = section.iter().fold(0, |acc, d| acc * 10 + d);
            if value == 0 {
                pending_zero = true;
                continue;
            }
            // 上一节是零、或本节不足四位（高位有零），补一个零
            if !text.is_empty() && (pending_zero || value < 1000) {
                text.push_str(self.digits[0]);
            }
            text.push_str(&self.section(section));
            text.push_str(SECTIONS[count - 1 - index]);
            pending_zero = false;
        }
        text
    }

    /// 一节（最多四位，非零）。中间的零合并成一个，末尾的零不写。
    fn section(&self, digits: &[usize]) -> String {
        let mut text = String::new();
        let mut zero = false;
        for (index, &d) in digits.iter().enumerate() {
            let unit = self.units[digits.len() - 1 - index];
            if d == 0 {
                zero = true;
                continue;
            }
            if zero && !text.is_empty() {
                text.push_str(self.digits[0]);
            }
            zero = false;
            text.push_str(self.digits[d]);
            text.push_str(unit);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_numerals() {
        assert_eq!(chinese_lower("0"), "零");
        assert_eq!(chinese_lower("10"), "十");
        assert_eq!(chinese_lower("15"), "十五");
        assert_eq!(chinese_lower("105"), "一百零五");
        assert_eq!(chinese_lower("110"), "一百一十");
        assert_eq!(chinese_lower("1010"), "一千零一十");
        assert_eq!(chinese_lower("10000"), "一万");
        assert_eq!(chinese_lower("100001"), "十万零一");
        assert_eq!(chinese_lower("20003000"), "二千万三千");
        assert_eq!(chinese_lower("20000300"), "二千万零三百");
        assert_eq!(chinese_lower("100000001"), "一亿零一");
        assert_eq!(chinese_lower("1500000000"), "十五亿");
        assert_eq!(chinese_lower("120000300"), "一亿二千万零三百");
        assert_eq!(
            chinese_lower("1234567890123"),
            "一万二千三百四十五亿六千七百八十九万零一百二十三"
        );
    }

    #[test]
    fn upper_numerals_keep_leading_one() {
        assert_eq!(chinese_upper("15"), "壹拾伍");
        assert_eq!(chinese_upper("123"), "壹佰贰拾叁");
        assert_eq!(chinese_upper("10086"), "壹万零捌拾陆");
    }
}
