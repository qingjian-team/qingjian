//! 建议列表里的一条。

#[derive(Debug, Clone, Copy)]
pub struct Suggestion<'a> {
    pub title: &'a str,

    /// 标题下面一行次要色的小字（命中的那段说明这类）。
    pub detail: Option<&'a str>,

    /// 右边一段次要色的小字（所在位置这类）。
    pub trailing: Option<&'a str>,
}
