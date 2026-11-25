use crate::sentence::Context;

/// 链里留几个词：记三元转移只用最后两个，多留几个是为了撤回时把上文一起退回去。
const CHAIN_HISTORY: usize = 4;

/// 连续上屏的链：记住最近上屏的几个中文词，下一个词上屏时就能记一条带前二词的转移（个人 n-gram 的三元）。
///
/// 标点、透传、回车上屏拼音、英文词等都打断链（下一个词按句首记）。
#[derive(Debug, Default, Clone)]
pub struct CommitChain {
    /// 最近上屏的中文词及其音节，最新的在末尾。
    ///
    /// 不只存两个：退格撤回一次选择要把刚上屏的那个词退掉，退掉之后「前一个词」「再前一个词」
    /// 都得接在它前面那个词上——只存两个的话，退掉就不知道该谁补位，下一个词记的转移会
    /// 挂在撤掉的那个字上。
    history: Vec<(String, Vec<String>)>,

    /// 上一个词上屏后缓冲区里还留着拼音：下一个词若紧接着从同一段拼音里选出，两个词本来是一起打的。
    same_buffer: bool,

    /// 当前这段拼音里已经上屏的词（文本与音节），整段选完时看要不要合成一个词。
    buffer_words: Vec<(String, Vec<String>)>,

    /// 这段拼音整段的学习键（按输入串记选择用的全部字母），第一个词上屏前记下。
    buffer_key: String,

    /// 这段拼音里组句内改过几次输入（退格删字母、删音节、删到光标后）。
    ///
    /// 改过说明用户中途换了主意：前面的词与后面的词不再是「一次连续输入」选出来的，
    /// [`Engine`](crate::engine::Engine) 的整段合成词到此为止（issue #1024）。
    buffer_edits: u32,
    /// 连续上屏的完整五笔词，按汉字总数限长，供下一次组词使用。
    code_words: Vec<String>,
}

impl CommitChain {
    /// 从链尾往前数第 `back` 个词（0 是上一个词）。
    fn word(&self, back: usize) -> Option<&(String, Vec<String>)> {
        self.history.iter().rev().nth(back)
    }

    /// 上一个词（若有）。
    pub fn previous(&self) -> Option<&str> {
        self.word(0).map(|(text, _)| text.as_str())
    }

    /// 再前一个词（若有）。
    fn earlier(&self) -> Option<&str> {
        self.word(1).map(|(text, _)| text.as_str())
    }

    /// 上一个词的音节。
    pub fn previous_syllables(&self) -> &[String] {
        self.word(0).map_or(&[], |(_, s)| s.as_slice())
    }

    /// 下一个词的上文：前一个词与再前一个词。
    pub fn context(&self) -> Context<'_> {
        Context {
            previous: self.previous(),
            earlier: self.earlier(),
        }
    }

    /// 链上是否有这个词（作为前一个或再前一个）。
    pub fn mentions(&self, text: &str) -> bool {
        self.previous() == Some(text) || self.earlier() == Some(text)
    }

    /// 下一个词是否与上一个词出自同一段拼音。
    pub fn same_buffer(&self) -> bool {
        self.same_buffer
    }

    /// 记下刚上屏的词；`buffer_left` 是上屏后缓冲区里是否还有拼音。
    pub fn advance(&mut self, text: &str, syllables: &[String], buffer_left: bool) {
        if !syllables.is_empty() {
            self.code_words.clear();
        }
        self.history.push((text.to_owned(), syllables.to_vec()));
        if self.history.len() > CHAIN_HISTORY {
            self.history.remove(0);
        }
        self.same_buffer = buffer_left;
        self.buffer_words
            .push((text.to_owned(), syllables.to_vec()));
    }

    /// 一段新拼音里的第一个词要上屏了：记下整段的学习键，上一段的词清掉。
    pub fn begin_buffer(&mut self, key: String) {
        self.buffer_words.clear();
        self.buffer_key = key;
        self.buffer_edits = 0;
    }

    /// 退格撤回最后一次选择：把刚上屏的那个词从这段拼音里退掉，**这段仍算同一段连续输入**。
    ///
    /// 撤回是「这一下选错了」，不是「这段拼音不打了」：接着在同一段拼音上重选、把它选完，
    /// `finish_buffer` 仍要把整段合成一个词（打 `jiaoyizhongxin` 时先选 交易、第二下选错撤回、
    /// 再重新选中心，整段仍要学成 交易中心）。所以这里只退掉那一个词，`buffer_key` 与已经选过的
    /// 词都留着；退到一个不剩才断开这段。
    ///
    /// 与「组句里改过输入就不合成」是两件事：那是用户改了拼音（[`Self::note_edit`]），
    /// 这是用户只改了这一下的选择，拼音没动。
    pub fn retract_buffer_word(&mut self) {
        let Some((retracted, _)) = self.buffer_words.pop() else {
            return;
        };
        // 撤回的那一下要跟没选过一样：链尾如果就是它，一并退掉，
        // 这样接着重选时的上文、以及下一个词记下的转移，都接在它前面那个词后面。
        if self.word(0).is_some_and(|(text, _)| *text == retracted) {
            self.history.pop();
        }
        if self.buffer_words.is_empty() {
            self.reset();
        }
    }

    /// 组句里改了一个字符（退格、删到光标后、删音节）：这段拼音被编辑过。
    /// 第一次改就把整段标记作废；同一段里改几次都算一次。
    pub fn note_edit(&mut self) {
        self.buffer_edits += 1;
    }

    /// 这段拼音自从最后一个词上屏以来有没有被编辑过。
    pub fn edited(&self) -> bool {
        self.buffer_edits > 0
    }

    /// 当前这段拼音里已上屏的词，按顺序。
    pub fn buffer_words(&self) -> &[(String, Vec<String>)] {
        &self.buffer_words
    }

    /// 当前这段拼音整段的学习键。
    pub fn buffer_key(&self) -> &str {
        &self.buffer_key
    }

    /// 打断链。
    pub fn reset(&mut self) {
        self.code_words.clear();
        self.history.clear();
        self.same_buffer = false;
        self.buffer_words.clear();
        self.buffer_edits = 0;
    }

    /// 缓冲区被清空或整段被别的东西吃掉：链不断，但下一个词不算同一段拼音。
    pub fn leave_buffer(&mut self) {
        self.same_buffer = false;
        self.buffer_words.clear();
    }

    pub fn clear_code_words(&mut self) {
        self.code_words.clear();
    }

    pub fn push_code_word(&mut self, text: &str, max_chars: usize) -> Vec<String> {
        self.code_words.push(text.to_owned());
        let mut chars = self
            .code_words
            .iter()
            .map(|word| word.chars().count())
            .sum::<usize>();
        while chars > max_chars {
            chars -= self.code_words.remove(0).chars().count();
        }
        (0..self.code_words.len().saturating_sub(1))
            .map(|start| self.code_words[start..].concat())
            .collect()
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
