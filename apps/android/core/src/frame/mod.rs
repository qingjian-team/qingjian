//! 交给 Kotlin 壳的一帧。
mod candidate;
mod sense;
mod voice;

use serde::Serialize;

pub use self::candidate::FrameCandidate;
pub use self::sense::FrameSense;
pub use self::voice::VoiceEvent;

/// 一次操作之后壳要做的事：先上屏 `commit`，再按 `preedit` / `candidates` 重画。JSON 过 JNI。
#[derive(Debug, Serialize)]
pub struct Frame {
    /// 这次操作要上屏的文本，没有为空。
    pub commit: String,

    /// 上屏前先删掉光标前几个字符（UTF-16 单位）：选算式结果时把算式换掉；平时为 0。
    pub replace: usize,

    /// 组句中的拼音显示形式（`kai'fa`），空表示不在组句。
    pub preedit: String,

    /// 一句话提示（删候选的结果），画在拼音行右侧，下一帧就没了。
    pub notice: String,

    /// 九宫格的拼音列：第一个没定下来的位置上的写法，最可能的在前；全键盘下为空。
    pub pinyin_choices: Vec<String>,

    /// 候选条上的候选，最多 [`Frame::MAX_CANDIDATES`] 条；下标就是 `Session::select` 的参数。
    pub candidates: Vec<FrameCandidate>,

    /// 组句处在算式（`v1+2`）或码点（`u4e00`）里：壳应把数字与运算符交进缓冲区，换数字键盘也不顶首选。
    pub expression: bool,
}

impl Frame {
    /// 一帧最多带的候选数：候选条一屏放不下这么多，展开面板做了再按需取后面的。
    pub const MAX_CANDIDATES: usize = 60;

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|error| {
            tracing::error!(%error, "帧序列化失败");
            String::from(
                r#"{"commit":"","replace":0,"preedit":"","notice":"","pinyin_choices":[],"candidates":[],"expression":false}"#,
            )
        })
    }
}
