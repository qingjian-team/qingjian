//! 送给选词模型后台线程的活。

use super::{ChoiceRequest, ComposeRequest};

pub(crate) enum Job {
    /// 给一页候选打分。
    Score(ChoiceRequest),

    /// 按音节出字，一次可以有几种切分。
    Compose(Vec<ComposeRequest>),
}
