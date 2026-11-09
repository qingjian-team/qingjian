//! 词码规则：码表只收了单字时，按词长取一条公式，从每个字的码里拼出词的码。
//!
//! 公式沿用 Rime `encoder` 的写法：大写字母选字、小写字母选码位，两两成对。
//! `A`–`T` 从前数（`A` 第一个字），`U`–`Z` 从后数（`Z` 最后一个字、`Y` 倒数第二个）；小写同理。
//! 例：`AaBa` 是两个字各取首码，`AaBaCaZa` 是前三字与末字各取首码。

use crate::error::DictionaryError;

/// 一个词最多拼出几条码：字有多条码时是各字的组合，多了只取前面的。
const MAX_DERIVED: usize = 16;

/// 公式里的一个下标：从前数或从后数，都从 0 起。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    Front(usize),

    Back(usize),
}

impl Pick {
    fn parse(c: char, first: char) -> Option<Self> {
        let offset = (c as u32).checked_sub(first as u32)? as usize;
        match offset {
            0..20 => Some(Self::Front(offset)),
            20..26 => Some(Self::Back(25 - offset)),
            _ => None,
        }
    }

    fn get<T: Copy>(self, items: &[T]) -> Option<T> {
        let index = match self {
            Self::Front(index) => index,
            Self::Back(index) => items.len().checked_sub(index + 1)?,
        };
        items.get(index).copied()
    }
}

/// 一条公式：(选字, 选码位) 依次拼起来。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Formula(Vec<(Pick, Pick)>);

impl Formula {
    fn parse(text: &str) -> Option<Self> {
        let chars: Vec<char> = text.trim().chars().collect();
        if chars.is_empty() || !chars.len().is_multiple_of(2) {
            return None;
        }
        chars
            .chunks(2)
            .map(|pair| Some((Pick::parse(pair[0], 'A')?, Pick::parse(pair[1], 'a')?)))
            .collect::<Option<Vec<_>>>()
            .map(Self)
    }

    /// 按选定的各字的码拼一条；选到的字或码位不存在时拼不出。
    fn apply(&self, codes: &[&str]) -> Option<String> {
        self.0
            .iter()
            .map(|(char_pick, code_pick)| {
                let code = char_pick.get(codes)?;
                code_pick.get(code.as_bytes()).map(char::from)
            })
            .collect()
    }
}

/// 一套方案的词码规则，按词长取公式；没写规则就不推导，码表里没有的词没有码。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WordRules {
    /// 按词长升序；最长的那条也管更长的词。
    rules: Vec<(usize, Formula)>,
}

impl WordRules {
    /// `(词长, 公式)` 读成规则。词长从 2 起；公式写错报错，说明是哪一条。
    pub fn parse<'a>(
        rules: impl IntoIterator<Item = (usize, &'a str)>,
    ) -> Result<Self, DictionaryError> {
        let mut parsed = Vec::new();
        for (length, text) in rules {
            if length < 2 {
                return Err(DictionaryError::InvalidWordRule(format!(
                    "{length} = {text:?}"
                )));
            }
            let formula = Formula::parse(text)
                .ok_or_else(|| DictionaryError::InvalidWordRule(format!("{length} = {text:?}")))?;
            parsed.push((length, formula));
        }
        parsed.sort_by_key(|(length, _)| *length);
        parsed.dedup_by_key(|(length, _)| *length);
        Ok(Self { rules: parsed })
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// 由各字的码拼出 `word` 的码，按各字码的先后组合，最多 [`MAX_DERIVED`] 条。
    /// 没有适用的公式、有字没有码时为空。
    pub(super) fn derive<'a>(
        &self,
        word: &str,
        codes_of: impl Fn(&str) -> Vec<&'a str>,
    ) -> Vec<String> {
        let chars: Vec<&str> = word
            .char_indices()
            .map(|(start, c)| &word[start..start + c.len_utf8()])
            .collect();
        let Some(formula) = self.formula(chars.len()) else {
            return Vec::new();
        };
        let per_char: Vec<Vec<&str>> = chars
            .iter()
            .map(|c| {
                let mut codes = codes_of(c);
                codes.truncate(MAX_DERIVED);
                codes
            })
            .collect();
        if per_char.iter().any(Vec::is_empty) {
            return Vec::new();
        }
        let mut derived = Vec::new();
        let mut choice = vec![0; per_char.len()];
        loop {
            let picked: Vec<&str> = choice.iter().zip(&per_char).map(|(i, c)| c[*i]).collect();
            if let Some(code) = formula.apply(&picked)
                && !derived.contains(&code)
            {
                derived.push(code);
                if derived.len() == MAX_DERIVED {
                    break;
                }
            }
            if !advance(&mut choice, &per_char) {
                break;
            }
        }
        derived
    }

    fn formula(&self, length: usize) -> Option<&Formula> {
        self.rules
            .iter()
            .rev()
            .find(|(min, _)| *min <= length)
            .map(|(_, formula)| formula)
    }
}

/// 组合计数往后走一步（末位先动）；走完了返回 `false`。
fn advance(choice: &mut [usize], options: &[Vec<&str>]) -> bool {
    for (slot, options) in choice.iter_mut().zip(options).rev() {
        *slot += 1;
        if *slot < options.len() {
            return true;
        }
        *slot = 0;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuxCodeLookup;
    use crate::AuxCodeTable;

    fn table(rules: &[(usize, &str)]) -> AuxCodeTable {
        AuxCodeTable::parse("开\tkd\n发\tfy\n发\tfz\n展\tzs\n开发\tkx\n")
            .unwrap()
            .with_word_rules(WordRules::parse(rules.iter().copied()).unwrap())
    }

    #[test]
    fn words_take_codes_from_their_characters() {
        let table = table(&[(2, "AaBa"), (3, "AaBaZz")]);
        assert_eq!(table.code_with_prefix("发展", "").as_deref(), Some("fz"));
        assert_eq!(
            table.code_with_prefix("展开发", "zk").as_deref(),
            Some("zky")
        );
        assert_eq!(AuxCodeLookup::codes_of(&table, "展发"), ["zf"]);
        assert_eq!(table.code_with_prefix("发展", "zz"), None);
    }

    #[test]
    fn listed_words_keep_their_own_codes() {
        let table = table(&[(2, "AaBa")]);
        assert_eq!(table.code_with_prefix("开发", "kf"), None);
        assert_eq!(table.code_with_prefix("开发", "k").as_deref(), Some("kx"));
    }

    #[test]
    fn the_longest_rule_covers_longer_words() {
        let table = table(&[(2, "AaZa")]);
        assert_eq!(
            table.code_with_prefix("开展发展", "").as_deref(),
            Some("kz")
        );
        assert_eq!(table.code_with_prefix("开", "").as_deref(), Some("kd"));
        assert_eq!(table.code_with_prefix("开没", ""), None);
    }

    #[test]
    fn several_codes_make_several_word_codes() {
        let rules = WordRules::parse([(2, "AaBb")]).unwrap();
        let derived = rules.derive("展发", |c| match c {
            "展" => vec!["zs"],
            _ => vec!["fy", "fz"],
        });
        assert_eq!(derived, ["zy", "zz"]);
    }

    #[test]
    fn back_picks_count_from_the_end() {
        let rules = WordRules::parse([(2, "ZzYa")]).unwrap();
        let derived = rules.derive("开展", |c| match c {
            "开" => vec!["kd"],
            _ => vec!["zs"],
        });
        assert_eq!(derived, ["sk"]);
    }

    #[test]
    fn bad_formulas_are_rejected() {
        for (length, text) in [(1, "Aa"), (2, "aA"), (2, "AaB"), (2, "")] {
            assert!(
                WordRules::parse([(length, text)]).is_err(),
                "{length} = {text}"
            );
        }
    }
}
