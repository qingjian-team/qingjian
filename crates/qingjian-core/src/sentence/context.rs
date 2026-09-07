/// 一个词的上文：前一个词与再前一个词。整句路径上的每一步、词级排序、个人 n-gram 的记录与打分都用它。
///
/// 句首词两个都是 `None`；句子第二个词 `previous` 有值、`earlier` 为 `None`（前词在句首）。
/// 静态模型只看 `previous`（bigram），个人 n-gram 两个都看（trigram）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Context<'a> {
    /// 前一个词；`None` 表示这个词在句首。
    pub previous: Option<&'a str>,

    /// 前一个词之前的那个词；`None` 表示前一个词在句首（或本身没有前词）。
    pub earlier: Option<&'a str>,
}

impl<'a> Context<'a> {
    /// 句首。
    pub const START: Self = Self {
        previous: None,
        earlier: None,
    };

    /// 只知道前一个词，它在句首。
    pub fn after(previous: &'a str) -> Self {
        Self {
            previous: Some(previous),
            earlier: None,
        }
    }

    /// 前两个词都知道：`earlier` 在前、`previous` 紧挨着这个词。
    pub fn after_two(earlier: &'a str, previous: &'a str) -> Self {
        Self {
            previous: Some(previous),
            earlier: Some(earlier),
        }
    }
}
