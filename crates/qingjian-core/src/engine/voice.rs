//! 语音上屏：各平台的识别结果（系统识别器、云端、离线模型）都从这里进 Core，与打字同样记历史与统计。

use super::Engine;
use super::commit::LastCommit;
use super::input_log::InputSource;

impl Engine {
    /// 语音识别出的一段文字上屏，返回交给应用的文字（繁体输出开着时转成繁体）。
    ///
    /// 记进输入历史（云联想的上文）、例句、输入日志与输入统计（词数按语言模型切，不算选词）；
    /// 不记词频：识别结果不是从候选里挑的。组句要由调用方先落下，这里不碰缓冲区。
    pub fn commit_voice(&mut self, text: &str) -> String {
        if text.trim().is_empty() {
            return String::new();
        }
        // 上一段组句的查询快照与这段话无关，别让输入日志把它记成这条的作用域
        *self.last_query.borrow_mut() = None;
        self.log_commit("", text, InputSource::Voice);
        self.meter_commit(text, InputSource::Voice, false);
        self.remember_commit(LastCommit::plain(text));
        self.punctuation.note_committed(text);
        self.note_text(text);
        self.chain.reset();
        self.traditional.convert(text)
    }
}
