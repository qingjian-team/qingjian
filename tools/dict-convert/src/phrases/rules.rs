//! 短语的边界规则：按整串的首尾字与成分词的形状筛掉「只是碰巧挨着」的组合。

/// 不能开头的字：助词、语气词。它们附在前一个词上，以它们开头的相邻组合（的是、了一）只是碰巧挨着。
const LEADING_CHARS: &str = "的了着得地吗吧呢啊呀哦嗯";

/// 不能结尾的字：附在后一个词上的副词 / 介词 / 连词，以及数词（是一、有两 只是句子的碎片）。
/// 就 / 在 不在这里：那就、放在 是人会整块打的。
const TRAILING_CHARS: &str = "不没很太也都又再才还把被和与及或而且但一二三四五六七八九十几两";

/// 人称代词：代词 + 了 / 这 / 那（你了、你这）是碎片；单字 + 的 里代词的（我的）总成词。
const PRONOUNS: &str = "我你他她它您咱";

/// 代词后面跟这些字不成词。
const AFTER_PRONOUN: &str = "了这那";

/// 系词 / 介词：它们后面跟单字（是新、在中）多半是碎片。
const COPULAS: &str = "是在有和与及或";

/// 单字 + 代词成词的那些单字（给我、帮你、想你、那我）；其余单字 + 代词（这你、天你、来你）是碎片。
const BEFORE_PRONOUN: &str = "给让帮等找叫送陪问骂爱想念疼怕怨为比跟和对被靠像信那";

/// 含这些的不是短语：笑声与语气词的黏连（哈哈好的、哈哈就）。
const NOISE: &str = "哈哈";

/// 这几个成分词拼起来能不能当短语。`strong` 是次数远超下限：够常见的组合只看首尾字，
/// 形状规则（单字 + 的、系词 + 单字）只用来筛次数刚过线的——好的 / 真的 / 是啥 是人天天打的，多的 / 岁的 / 是新 不是。
pub fn passes(parts: &[&str], strong: bool) -> bool {
    let text: String = parts.concat();
    let (Some(first), Some(last)) = (text.chars().next(), text.chars().last()) else {
        return false;
    };
    if LEADING_CHARS.contains(first) || TRAILING_CHARS.contains(last) {
        return false;
    }
    // 叠字（哈哈哈）另算，不是短语；笑声黏上的（哈哈好的）也不是
    if text.chars().all(|c| c == first) || text.contains(NOISE) {
        return false;
    }
    let single = |part: &str| part.chars().count() == 1;
    let [head @ .., tail] = parts else {
        return false;
    };
    let head_chars: usize = head.iter().map(|p| p.chars().count()).sum();
    // 代词 + 了 / 这 / 那（你了、你这）是碎片
    if head_chars == 1 && PRONOUNS.contains(first) && single(tail) && AFTER_PRONOUN.contains(last) {
        return false;
    }
    // 单字 + 代词：给我 / 帮你 成词，这你 / 天你 / 来你 是碎片
    if head_chars == 1 && single(tail) && PRONOUNS.contains(last) && !BEFORE_PRONOUN.contains(first)
    {
        return false;
    }
    if strong {
        return true;
    }
    // 单字 + 的：代词成词（我的），其余次数不够的（岁的 / 前的）是碎片
    if *tail == "的" && head_chars == 1 && !PRONOUNS.contains(first) {
        return false;
    }
    // 是 / 在 + 单字（是新、在中）是碎片
    if parts.len() == 2 && single(parts[0]) && COPULAS.contains(first) && single(tail) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_rules() {
        assert!(passes(&["我", "的"], false));
        assert!(passes(&["都", "是"], false));
        assert!(passes(&["那", "就"], false));
        assert!(passes(&["放", "在"], false));
        assert!(passes(&["不", "知道"], false));
        assert!(passes(&["给", "我"], false));
        assert!(passes(&["自己", "的"], false));
        assert!(!passes(&["的", "是"], true));
        assert!(!passes(&["了", "一"], true));
        assert!(!passes(&["是", "一"], true));
        assert!(!passes(&["我", "很"], true));
        assert!(!passes(&["哈", "哈", "哈"], true));
        assert!(!passes(&["你", "了"], true));
        assert!(!passes(&["你", "这"], true));
        assert!(!passes(&["这", "你"], true));
        assert!(!passes(&["哈哈", "好", "的"], true));
        // 形状规则只筛次数刚过线的：好的 常见、岁的 不常见
        assert!(passes(&["好", "的"], true));
        assert!(!passes(&["岁", "的"], false));
        assert!(passes(&["是", "啥"], true));
        assert!(!passes(&["是", "新"], false));
        assert!(!passes(&[], true));
    }
}
