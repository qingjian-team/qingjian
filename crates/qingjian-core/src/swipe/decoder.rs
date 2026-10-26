//! 解码接口。

use super::{SwipeKey, SwipeLexicon, SwipePoint};

/// 轨迹 → 字母串。候选词由调用方按当前在打什么给（拼音的音节串、英文词），解码只管比；换实现壳与 Session 不用改。
pub trait SwipeDecoder: Send {
    /// 按可能性从高到低给几个字母串（小写）；轨迹或键位是空的回空。
    fn decode(
        &self,
        trace: &[SwipePoint],
        keys: &[SwipeKey],
        lexicon: &SwipeLexicon,
    ) -> Vec<String>;
}
