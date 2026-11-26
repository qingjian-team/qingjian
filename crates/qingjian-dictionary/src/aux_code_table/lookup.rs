//! 辅码查询接口：Core 只看这一个 trait，不认识码表文件。

use std::borrow::Cow;

/// 辅码查询：Core 只认这个接口，不认识码表文件。过滤是反向的：拿候选词逐个问有没有以码段开头的码
/// （设计见 `docs/design/aux-code.md`）。
pub trait AuxCodeLookup: Send + Sync {
    /// `word` 的码里第一个以 `prefix` 开头的，返回那条码本身（显示码取的就是它）。
    ///
    /// 同一个词有多条码时取文件里的第一条；表里没有、按方案的词码规则推导出来的是新造的串。
    /// 多张码表叠加时由调用方按表序取先命中的那张。
    /// 没命中返回 `None`。`prefix` 为空时任何码都算命中。
    fn code_with_prefix<'a>(&'a self, word: &str, prefix: &str) -> Option<Cow<'a, str>>;

    /// `word` 的全部码，按表里的顺序；没有这个词时为空。
    ///
    /// 只给「查看码」这类诊断用：过滤走 `code_with_prefix`，热路径上不必枚举。
    fn codes_of<'a>(&'a self, word: &str) -> Vec<Cow<'a, str>>;

    /// 容错版 [`Self::code_with_prefix`]：码段打错一位也算命中，返回命中的码与差了几位。
    ///
    /// 精确命中的距离是 0，调用方据此把「码没错」的排在「码打错了」的前面。
    /// `max_distance` 一般给 1 —— 码段本来就短（两三笔），容两位等于没容错。
    fn code_within<'a>(
        &'a self,
        word: &str,
        prefix: &str,
        max_distance: usize,
    ) -> Option<(Cow<'a, str>, usize)> {
        if let Some(code) = self.code_with_prefix(word, prefix) {
            return Some((code, 0));
        }
        let mut best: Option<(Cow<'a, str>, usize)> = None;
        for code in self.codes_of(word) {
            let head_len = code.len().min(prefix.len());
            let distance = edit_distance(prefix, &code[..head_len]);
            if distance == 0 || distance > max_distance {
                continue;
            }
            if best.as_ref().is_none_or(|(_, seen)| distance < *seen) {
                best = Some((code, distance));
            }
        }
        best
    }
}

/// 只用于短码段的编辑距离（码段一般两三个字符）：标准 Levenshtein，按字节比 —— 辅码都是 ASCII。
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for (i, left) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, right) in b.iter().enumerate() {
            let cost = usize::from(left != right);
            current[j + 1] = (current[j] + 1)
                .min(previous[j + 1] + 1)
                .min(previous[j] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}
