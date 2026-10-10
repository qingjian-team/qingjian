use super::cloud_word::CloudWord;

/// 联想结果。云端词按模型顺序优先显示，本地同文候选在布局中合并。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prediction {
    /// 对应的请求序号。
    pub sequence: u64,

    /// 这段拼音可能对应的词（已按拼音校验）。
    pub words: Vec<CloudWord>,

    /// 组句中的整句补全，替换整段拼音。
    pub sentence: Option<String>,
}

impl Prediction {
    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.sentence.is_none()
    }
}
