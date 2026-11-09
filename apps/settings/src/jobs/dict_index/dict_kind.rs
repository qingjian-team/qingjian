//! 清单里一本词库的种类，对应词库仓库 `dict.toml` 的 `kind`。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DictKind {
    /// 基础词库，随包装。
    Base,

    /// 附加词库（领域、通用大词库、第三方），装进用户 `dicts/`。
    Extra,

    /// 释义表，装进用户 `glossaries/`。
    Glossary,

    /// 词表（英文、越南语），随包装。
    Wordlist,

    /// 这一版输入法不认得的种类（词库仓库后来加的）：不列，也不让整份清单读不了。
    #[serde(other)]
    Other,
}
