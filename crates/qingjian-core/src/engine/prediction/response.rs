use super::cloud_word::CloudWord;
use super::sentence_translation::SentenceTranslation;

/// 联想结果。只是候选之外的补充展示，**不重排本地候选**：云端词补进第一页末尾几格。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prediction {
    /// 对应的请求序号。
    pub sequence: u64,

    /// 这段拼音可能对应的词（已按拼音校验）。
    pub words: Vec<CloudWord>,

    /// 组句中的整句补全，替换整段拼音。
    pub sentence: Option<String>,

    /// 整句候选的译文，不改变中文候选顺序。
    pub translation: Option<SentenceTranslation>,

    /// 云服务请求失败；保留原文，供评审窗口提示。
    pub failed: bool,
}

impl Prediction {
    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.sentence.is_none() && self.translation.is_none()
    }
}
