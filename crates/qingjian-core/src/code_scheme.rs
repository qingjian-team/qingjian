//! 形码方案：一张码表加上按键规则（满码唯一自动上屏、顶字）。
//!
//! 规则只在只用形码时生效：混输下码长以外的字母还要留给拼音，满码时多敲一个键不能当成顶字。

use qingjian_dictionary::CodeTable;

/// 一套形码方案。内置的 86 五笔与用户自己导入的方案都是它，区别只在码表与开关。
#[derive(Debug, Clone)]
pub struct CodeScheme {
    /// 输入日志里的方案键，回放靠它认出这几行是形码。86 五笔是 `wubi`（沿用老日志的写法）。
    pub key: String,

    pub table: CodeTable,

    /// 满码几个字母：两条按键规则与混输下「超出码长只剩拼音」按它算。缺省取码表最常见的编码长度。
    pub code_length: usize,

    /// 编码敲满码长、而且这个全码只对应一个词时，直接上屏。
    pub auto_commit: bool,

    /// 编码已经满码长时再敲一个字母：先把高亮的候选上屏，这个字母起新的一段编码。
    pub top: bool,

    /// 是否声明兼容 86 五笔取码规则，允许用单字全码生成连续上屏词的编码。
    pub wubi_phrases: bool,
}

impl CodeScheme {
    /// 规则全关的方案；开关由调用方按方案描述再设。
    pub fn new(key: impl Into<String>, table: CodeTable) -> Self {
        Self {
            key: key.into(),
            code_length: table.full_code_len(),
            table,
            auto_commit: false,
            top: false,
            wubi_phrases: false,
        }
    }

    /// 按 86 五笔规则取 2～10 字词码。未声明规则或缺少单字编码时不造词。
    pub fn phrase_code(&self, text: &str) -> Option<String> {
        if !self.wubi_phrases {
            return None;
        }
        let codes = text
            .chars()
            .map(|character| self.table.full_code(character))
            .collect::<Option<Vec<_>>>()?;
        match codes.as_slice() {
            [first, second] => Some(format!("{}{}", first.get(..2)?, second.get(..2)?)),
            [first, second, third] => Some(format!(
                "{}{}{}",
                first.get(..1)?,
                second.get(..1)?,
                third.get(..2)?
            )),
            [first, second, third, .., last] if codes.len() <= 10 => Some(format!(
                "{}{}{}{}",
                first.get(..1)?,
                second.get(..1)?,
                third.get(..1)?,
                last.get(..1)?
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CodeScheme;
    use qingjian_dictionary::CodeTable;

    #[test]
    fn character_codes_generate_standard_wubi_phrase_codes() {
        let table = CodeTable::parse(
            "中\tkhk\t1\n国\tlgyi\t1\n青\tgef\t1\n简\ttujf\t1\n输\tlwgj\t1\n入\ttty\t1\n法\tifcy\t1\n华\twxfj\t1\n人\twwww\t1\n民\tnav\t1\n共\tawu\t1\n和\ttkg\t1\n",
        )
        .unwrap();
        let mut scheme = CodeScheme::new("wubi", table);
        scheme.wubi_phrases = true;
        assert_eq!(scheme.phrase_code("中国").as_deref(), Some("khlg"));
        assert_eq!(scheme.phrase_code("青简").as_deref(), Some("getu"));
        assert_eq!(scheme.phrase_code("输入法").as_deref(), Some("ltif"));
        assert_eq!(
            scheme.phrase_code("中华人民共和国").as_deref(),
            Some("kwwl")
        );
    }

    #[test]
    fn phrase_codes_use_full_character_codes_not_shortcuts() {
        let table = CodeTable::parse(
            "甲\ta\t100\n甲\tabcd\t10\n乙\tefgh\t10\n丙\tijkl\t10\n丁\tmnop\t10\n戊\tqrst\t10\n",
        )
        .unwrap();
        let mut scheme = CodeScheme::new("custom", table);
        assert_eq!(scheme.phrase_code("甲乙"), None);
        scheme.wubi_phrases = true;
        assert_eq!(scheme.phrase_code("甲乙").as_deref(), Some("abef"));
        assert_eq!(scheme.phrase_code("甲乙丙").as_deref(), Some("aeij"));
        assert_eq!(scheme.phrase_code("甲乙丙丁").as_deref(), Some("aeim"));
        assert_eq!(
            scheme.phrase_code("甲乙丙丁戊甲乙丙丁戊").as_deref(),
            Some("aeiq")
        );
        assert_eq!(scheme.phrase_code("甲"), None);
        assert_eq!(scheme.phrase_code("甲乙丙丁戊甲乙丙丁戊甲"), None);
        assert_eq!(scheme.phrase_code("甲？"), None);
    }
}
