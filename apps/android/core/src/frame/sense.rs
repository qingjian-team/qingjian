//! 帧里候选的一条译文。
use qingjian_core::Sense;
use serde::Serialize;

/// 候选条下面那行小字与长按气泡里的一项。
#[derive(Debug, Serialize)]
pub struct FrameSense {
    /// 词性缩写（`n.`）。
    pub pos: Option<&'static str>,

    pub text: String,

    /// 读音（日语假名）。
    pub reading: Option<String>,

    /// 还是生词（壳用强调色画）。
    pub fresh: bool,
}

impl From<&Sense> for FrameSense {
    fn from(sense: &Sense) -> Self {
        Self {
            pos: sense.part_of_speech.map(|pos| pos.abbreviation()),
            text: sense.text.clone(),
            reading: sense.reading.clone(),
            fresh: sense.fresh,
        }
    }
}
