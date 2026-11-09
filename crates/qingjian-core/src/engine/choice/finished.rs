//! 选词模型后台线程交回的结果。

use std::sync::Arc;

use super::{CharTable, ChoiceRequest};

pub(crate) enum Finished {
    /// 一页候选的分：请求原样带回，壳拿它对上缓存。
    Scored {
        request: ChoiceRequest,
        logits: Vec<f32>,
        ms: u64,
    },

    /// 按音节出的字分布，按请求的缓存键；`None` 是模型出不了。
    Composed(Vec<(String, Option<Arc<CharTable>>)>),
}
