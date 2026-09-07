use serde::{Deserialize, Serialize};

use super::part_of_speech::PartOfSpeech;

/// 一条释义：词性 + 目标语言文本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sense {
    /// 该释义下的词性。数据源没有词性信息时为 `None`（如 CC-CEDICT），候选框只显示译文。
    pub part_of_speech: Option<PartOfSpeech>,

    /// 学习语言的译文，短语级别，不是整段解释。
    pub text: String,

    /// 译文的读音（日语假名），给不认识汉字读法的人看；英文等没有。
    pub reading: Option<String>,

    /// 生词：这条译词用户在候选里还没见过几轮（`Engine::annotate` 按词汇记录填，释义表里恒为 false），壳可以标出来。
    #[serde(default)]
    pub fresh: bool,
}

impl Sense {
    /// 译文按汉字段配上假名（振り仮名）；没有读音时只有译文本身一段。
    pub fn furigana(&self) -> Vec<super::FuriganaSegment> {
        match &self.reading {
            Some(reading) => super::furigana(&self.text, reading),
            None => vec![super::FuriganaSegment {
                text: self.text.clone(),
                reading: None,
            }],
        }
    }
}
