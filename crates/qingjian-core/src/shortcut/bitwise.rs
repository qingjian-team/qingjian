//! `vbit` 子模式：定宽整数的位运算。
//!
//! 语法照 C++：`<<` `>>` `&` `|` `^` `~`，也认写出来的英文（`shl` `shr` `and` `or` `xor` `not`）。
//! 字面量认 `0x` / `0b` / `0o` / 十进制。位宽与符号由子模式名给（见 [`BitWidth`]），
//! 结果按位宽截断、有符号位按补码解释，和 C++ 里 `int8_t` / `uint32_t` 的行为一致。
//!
//! 越界不 panic：除数为零、移位位数超过位宽这类在 C++ 里是未定义行为，这里返回 `None`，
//! 候选里就不出东西，用户看到的是输入原样。

use crate::shortcut::numeral;

/// 位宽与符号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitWidth {
    /// 总位数，`1..=128`。
    pub bits: u32,

    /// 有符号（补码解释最高位）。
    pub signed: bool,
}

impl BitWidth {
    /// `bit` / `bitu` 缺省的 64 位，与 C++ 的 `long long` / `unsigned long long` 对齐。
    pub const DEFAULT: Self = Self {
        bits: 64,
        signed: true,
    };

    /// 该位宽下的掩码；128 位就是全 1。
    fn mask(self) -> u128 {
        if self.bits >= 128 {
            u128::MAX
        } else {
            (1u128 << self.bits) - 1
        }
    }

    /// 按位宽截断。
    fn wrap(self, value: u128) -> u128 {
        value & self.mask()
    }

    /// 补码还原成有符号值（无符号位宽就是原值）。
    fn decode(self, value: u128) -> i128 {
        let value = self.wrap(value);
        if !self.signed || self.bits >= 128 {
            return value as i128;
        }
        let sign = 1u128 << (self.bits - 1);
        if value & sign == 0 {
            value as i128
        } else {
            (value as i128) - (1i128 << self.bits)
        }
    }

    /// 十进制写法（有符号带负号）。
    pub fn decimal(self, value: u128) -> String {
        let value = self.wrap(value);
        // 无符号直接给原值：128 位的 `not0` 是 `u128::MAX`，走 `decode` 的 `as i128` 会变成 -1
        if !self.signed {
            return value.to_string();
        }
        self.decode(value).to_string()
    }

    /// `0x` 十六进制：按位宽补零到 2 / 4 / 8 / 16 / 32 位（8 位宽两位、16 位宽四位，依此类推）。
    pub fn hex(self, value: u128) -> String {
        let digits = match self.bits {
            0..=8 => 2,
            9..=16 => 4,
            17..=32 => 8,
            33..=64 => 16,
            _ => 32,
        };
        format!("0x{:0width$x}", self.wrap(value), width = digits)
    }

    /// `0b` 二进制，不补零：128 位补满要 128 个字符，候选里放不下。
    pub fn binary(self, value: u128) -> String {
        let text = format!("{:b}", self.wrap(value));
        format!("0b{}", text)
    }

    /// 名字反过来拼回子模式（`bit8u` / `bit128`），给候选上的说明用。
    pub fn label(self) -> String {
        if self.bits == 64 {
            if self.signed {
                "bit".to_owned()
            } else {
                "bitu".to_owned()
            }
        } else if self.signed {
            format!("bit{}", self.bits)
        } else {
            format!("bit{}u", self.bits)
        }
    }
}

/// 位运算求值结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitResult {
    /// 原始值（已按位宽截断）。
    pub raw: u128,

    /// 位宽与符号。
    pub width: BitWidth,
}

impl BitResult {
    /// 候选里给的几种写法，顺序：十进制、十六进制、二进制、中文小写、中文大写。
    pub fn forms(&self) -> Vec<String> {
        let decimal = self.width.decimal(self.raw);
        let mut forms = vec![
            decimal.clone(),
            self.width.hex(self.raw),
            self.width.binary(self.raw),
        ];
        // 中文数字只对非负值有意义（负数读法各家不一，不硬凑）
        if !decimal.starts_with('-') {
            forms.push(numeral::chinese_lower(&decimal));
            forms.push(numeral::chinese_upper(&decimal));
        }
        forms
    }
}

/// 算一条位运算表达式。位宽缺省 [`BitWidth::DEFAULT`]。
pub fn evaluate(text: &str, width: BitWidth) -> Option<BitResult> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        pos: 0,
        width,
    };
    let raw = parser.expression()?;
    parser.skip_spaces();
    if parser.pos != parser.chars.len() {
        return None;
    }
    Some(BitResult { raw, width })
}

struct Parser {
    /// 输入按字符拆开，按下标前进。
    chars: Vec<char>,

    /// 当前下标。
    pos: usize,

    /// 位宽与符号。
    width: BitWidth,
}

impl Parser {
    fn expression(&mut self) -> Option<u128> {
        let mut value = self.xor()?;
        loop {
            self.skip_spaces();
            if self.eat_char('|') || self.eat_word("or") {
                let rhs = self.xor()?;
                value = self.width.wrap(value | rhs);
            } else {
                return Some(value);
            }
        }
    }

    fn xor(&mut self) -> Option<u128> {
        let mut value = self.and()?;
        loop {
            self.skip_spaces();
            if self.eat_char('^') || self.eat_word("xor") {
                let rhs = self.and()?;
                value = self.width.wrap(value ^ rhs);
            } else {
                return Some(value);
            }
        }
    }

    fn and(&mut self) -> Option<u128> {
        let mut value = self.shift()?;
        loop {
            self.skip_spaces();
            if self.eat_char('&') || self.eat_word("and") {
                let rhs = self.shift()?;
                value = self.width.wrap(value & rhs);
            } else {
                return Some(value);
            }
        }
    }

    fn shift(&mut self) -> Option<u128> {
        let mut value = self.additive()?;
        loop {
            self.skip_spaces();
            if self.eat_str("<<") || self.eat_word("shl") {
                let rhs = self.additive()?;
                // 移位位数到或超过位宽在 C++ 里是未定义行为，这里直接不给结果
                if rhs >= u128::from(self.width.bits) {
                    return None;
                }
                value = self.width.wrap(value << rhs);
            } else if self.eat_str(">>") || self.eat_word("shr") {
                let rhs = self.additive()?;
                if rhs >= u128::from(self.width.bits) {
                    return None;
                }
                if self.width.signed {
                    // 有符号右移按算术右移（补符号位），与 C++ 的实现定义一致
                    let extended = self.width.decode(value) >> rhs;
                    value = self.width.wrap(extended as u128);
                } else {
                    value = self.width.wrap(value >> rhs);
                }
            } else {
                return Some(value);
            }
        }
    }

    /// `+ -`：与 C++ 同优先级，低于 `* / %`、高于 `<< >>`。
    fn additive(&mut self) -> Option<u128> {
        let mut value = self.term()?;
        loop {
            self.skip_spaces();
            if self.eat_char('+') {
                let rhs = self.term()?;
                value = self.width.wrap(value.wrapping_add(rhs));
            } else if self.eat_char('-') {
                let rhs = self.term()?;
                value = self.width.wrap(value.wrapping_sub(rhs));
            } else {
                return Some(value);
            }
        }
    }

    /// `* / %`。除数为 0 在 C++ 里是未定义行为，这里直接不给结果。
    fn term(&mut self) -> Option<u128> {
        let mut value = self.unary()?;
        loop {
            self.skip_spaces();
            if self.eat_char('*') {
                let rhs = self.unary()?;
                value = self.width.wrap(value.wrapping_mul(rhs));
            } else if self.eat_char('/') {
                let rhs = self.unary()?;
                if rhs == 0 {
                    return None;
                }
                value = self.width.wrap(value / rhs);
            } else if self.eat_char('%') {
                let rhs = self.unary()?;
                if rhs == 0 {
                    return None;
                }
                value = self.width.wrap(value % rhs);
            } else {
                return Some(value);
            }
        }
    }

    fn unary(&mut self) -> Option<u128> {
        self.skip_spaces();
        if self.eat_char('~') || self.eat_word("not") {
            let value = self.unary()?;
            return Some(self.width.wrap(!value));
        }
        if self.eat_char('-') {
            let value = self.unary()?;
            return Some(self.width.wrap(value.wrapping_neg()));
        }
        if self.eat_char('+') {
            return self.unary();
        }
        self.primary()
    }

    fn primary(&mut self) -> Option<u128> {
        self.skip_spaces();
        if self.eat_char('(') {
            let value = self.expression()?;
            self.skip_spaces();
            if !self.eat_char(')') {
                return None;
            }
            return Some(value);
        }
        self.number()
    }

    fn number(&mut self) -> Option<u128> {
        self.skip_spaces();
        let start = self.pos;
        // 输入法里没有空格可用（空格是上屏键），运算符要和数字连写：`1shl10`、`0xffand3`。
        // 所以每读一个字符前先看这里是不是一个运算符词的开头，是就收住数字。
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if !(c.is_ascii_alphanumeric() || c == '_') {
                break;
            }
            if self.pos > start && self.at_operator_word() {
                break;
            }
            self.pos += 1;
        }
        if start == self.pos {
            return None;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        let text = text.replace('_', "");
        let value = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            u128::from_str_radix(hex, 16).ok()?
        } else if let Some(bin) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
            u128::from_str_radix(bin, 2).ok()?
        } else if let Some(oct) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
            u128::from_str_radix(oct, 8).ok()?
        } else {
            text.parse::<u128>().ok()?
        };
        Some(self.width.wrap(value))
    }

    /// 当前位置是不是某个运算符词的起点（只探测，不前进）。
    fn at_operator_word(&self) -> bool {
        ["and", "or", "xor", "not", "shl", "shr"]
            .iter()
            .any(|word| self.matches_word(word))
    }

    /// 从当前下标起能不能完整匹配 `word`，且后面不紧跟字母（`android` 不算 `and`，`and3` 算）。
    fn matches_word(&self, word: &str) -> bool {
        let len = word.chars().count();
        if self.pos + len > self.chars.len() {
            return false;
        }
        let head = word
            .chars()
            .enumerate()
            .all(|(index, c)| self.chars[self.pos + index] == c);
        if !head {
            return false;
        }
        !matches!(self.chars.get(self.pos + len), Some(c) if c.is_ascii_alphabetic())
    }

    fn skip_spaces(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn eat_char(&mut self, c: char) -> bool {
        if self.chars.get(self.pos) == Some(&c) {
            self.pos += 1;
            return true;
        }
        false
    }

    fn eat_str(&mut self, s: &str) -> bool {
        let len = s.chars().count();
        if self.pos + len > self.chars.len() {
            return false;
        }
        let matched = s
            .chars()
            .enumerate()
            .all(|(index, c)| self.chars[self.pos + index] == c);
        if matched {
            self.pos += len;
        }
        matched
    }

    /// 整词匹配（`and` / `or` 这类），后面不能紧跟字母，免得 `android` 被当成 `and`。
    /// 后跟数字是可以的：输入法里没有空格，`and3`、`not0xff` 这类连写是常态。
    fn eat_word(&mut self, word: &str) -> bool {
        if !self.matches_word(word) {
            return false;
        }
        self.pos += word.chars().count();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(text: &str) -> Option<BitResult> {
        evaluate(text, BitWidth::DEFAULT)
    }

    #[test]
    fn cpp_operators_match_rust_bit_ops() {
        assert_eq!(eval("0b1010 ^ 0b1100").unwrap().raw, 0b0110);
        assert_eq!(eval("0xF0 & 0x3C").unwrap().raw, 0x30);
        assert_eq!(eval("0xF0 | 0x0F").unwrap().raw, 0xFF);
        assert_eq!(eval("1 << 10").unwrap().raw, 1024);
        assert_eq!(eval("0xFFFF >> 4").unwrap().raw, 0x0FFF);
        assert_eq!(eval("~0").unwrap().raw, u64::MAX as u128);
    }

    #[test]
    fn english_spellings_work() {
        assert_eq!(eval("0b1010 xor 0b1100").unwrap().raw, 0b0110);
        assert_eq!(eval("0xF0 and 0x3C").unwrap().raw, 0x30);
        assert_eq!(eval("0xF0 or 0x0F").unwrap().raw, 0xFF);
        assert_eq!(eval("1 shl 8").unwrap().raw, 256);
        assert_eq!(eval("256 shr 4").unwrap().raw, 16);
        assert_eq!(eval("not 0").unwrap().raw, u64::MAX as u128);
    }

    #[test]
    fn english_spellings_without_spaces() {
        // 输入法里空格是上屏键，运算符只能和数字连写
        assert_eq!(eval("1shl10").unwrap().raw, 1024);
        assert_eq!(eval("256shr4").unwrap().raw, 16);
        assert_eq!(eval("0xffand3").unwrap().raw, 3);
        assert_eq!(eval("0b1010xor0b1100").unwrap().raw, 0b0110);
        assert_eq!(eval("1or2").unwrap().raw, 3);
        assert_eq!(eval("not0").unwrap().raw, u64::MAX as u128);
        assert_eq!(eval("1shl8xor3").unwrap().raw, (1u128 << 8) ^ 3);
        assert_eq!(eval("0xfaand0xff").unwrap().raw, 0xfa);
        assert_eq!(eval("(1shl4)or1").unwrap().raw, 17);
    }

    #[test]
    fn widths_truncate_and_decode_sign() {
        let byte = BitWidth {
            bits: 8,
            signed: true,
        };
        let value = evaluate("0x1FF", byte).unwrap();
        assert_eq!(byte.decimal(value.raw), "-1");
        assert_eq!(byte.hex(value.raw), "0xff");

        let unsigned = BitWidth {
            bits: 8,
            signed: false,
        };
        assert_eq!(unsigned.decimal(0xFF), "255");

        let word = BitWidth {
            bits: 32,
            signed: true,
        };
        assert_eq!(word.decimal(0xFFFF_FFFF), "-1");
        assert_eq!(word.hex(0xFFFF_FFFF), "0xffffffff");
    }

    #[test]
    fn out_of_range_shifts_and_garbage_are_rejected() {
        assert!(eval("1 << 64").is_none());
        assert!(eval("1 << 63").is_some());
        assert!(eval("1 +").is_none());
        assert!(eval("").is_none());
        assert!(eval("(1").is_none());
    }

    #[test]
    fn forms_include_chinese_numerals() {
        let result = eval("1 << 10").unwrap();
        let forms = result.forms();
        assert_eq!(forms[0], "1024");
        assert_eq!(forms[1], "0x0000000000000400");
        assert!(forms.iter().any(|f| f == "一千零二十四"));
        assert!(forms.iter().any(|f| f == "壹仟零贰拾肆"));
    }
}
