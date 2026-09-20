//! 辅码查询接口：Core 只看这一个 trait，不认识码表文件。

/// 辅码查询：Core 只认这个接口，不认识码表文件。过滤是反向的：拿候选词逐个问有没有以码段开头的码
/// （设计见 `docs/design/aux-code.md`）。
pub trait AuxCodeLookup: Send + Sync {
    /// `word` 的码里第一个以 `prefix` 开头的，返回那条码本身（显示码取的就是它）。
    ///
    /// 同一个词有多条码时取文件里的第一条；多张码表叠加时由调用方按表序取先命中的那张。
    /// 没命中返回 `None`。`prefix` 为空时任何码都算命中。
    fn code_with_prefix<'a>(&'a self, word: &str, prefix: &str) -> Option<&'a str>;

    /// `word` 的全部码，按表里的顺序；没有这个词时为空。
    ///
    /// 只给「查看码」这类诊断用：过滤走 `code_with_prefix`，热路径上不必枚举。
    fn codes_of<'a>(&'a self, word: &str) -> Vec<&'a str>;
}
