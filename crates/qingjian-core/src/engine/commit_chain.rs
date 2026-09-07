use crate::sentence::Context;

/// 连续上屏的链：记住最近上屏的两个中文词，下一个词上屏时就能记一条带前二词的转移（个人 n-gram 的三元）。
///
/// 标点、透传、回车上屏拼音、英文词等都打断链（下一个词按句首记）。
#[derive(Debug, Default, Clone)]
pub struct CommitChain {
    /// 上一个上屏的中文词及其音节；`None` 表示下一个词在句首。
    previous: Option<(String, Vec<String>)>,

    /// 上一个词之前的那个词；`None` 表示上一个词在句首。
    earlier: Option<String>,

    /// 上一个词上屏后缓冲区里还留着拼音：下一个词若紧接着从同一段拼音里选出，两个词本来是一起打的。
    same_buffer: bool,
}

impl CommitChain {
    /// 上一个词（若有）。
    pub fn previous(&self) -> Option<&str> {
        self.previous.as_ref().map(|(text, _)| text.as_str())
    }

    /// 上一个词的音节。
    pub fn previous_syllables(&self) -> &[String] {
        self.previous.as_ref().map_or(&[], |(_, s)| s.as_slice())
    }

    /// 下一个词的上文：前一个词与再前一个词。
    pub fn context(&self) -> Context<'_> {
        Context {
            previous: self.previous(),
            earlier: self.earlier.as_deref(),
        }
    }

    /// 链上是否有这个词（作为前一个或再前一个）。
    pub fn mentions(&self, text: &str) -> bool {
        self.previous() == Some(text) || self.earlier.as_deref() == Some(text)
    }

    /// 下一个词是否与上一个词出自同一段拼音。
    pub fn same_buffer(&self) -> bool {
        self.same_buffer
    }

    /// 记下刚上屏的词；`buffer_left` 是上屏后缓冲区里是否还有拼音。
    pub fn advance(&mut self, text: &str, syllables: &[String], buffer_left: bool) {
        self.earlier = self.previous.take().map(|(text, _)| text);
        self.previous = Some((text.to_owned(), syllables.to_vec()));
        self.same_buffer = buffer_left;
    }

    /// 打断链。
    pub fn reset(&mut self) {
        self.previous = None;
        self.earlier = None;
        self.same_buffer = false;
    }

    /// 缓冲区被清空或整段被别的东西吃掉：链不断，但下一个词不算同一段拼音。
    pub fn leave_buffer(&mut self) {
        self.same_buffer = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_last_two_words_as_context() {
        let mut chain = CommitChain::default();
        assert_eq!(chain.context(), Context::START);
        chain.advance("我", &["wo".into()], true);
        assert_eq!(chain.context(), Context::after("我"));
        chain.advance("想", &["xiang".into()], false);
        assert_eq!(chain.context(), Context::after_two("我", "想"));
        chain.advance("去", &["qu".into()], false);
        assert_eq!(chain.context(), Context::after_two("想", "去"));
        assert!(chain.mentions("想") && chain.mentions("去") && !chain.mentions("我"));
        chain.reset();
        assert_eq!(chain.context(), Context::START);
    }
}
