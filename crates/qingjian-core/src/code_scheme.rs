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
        }
    }
}
