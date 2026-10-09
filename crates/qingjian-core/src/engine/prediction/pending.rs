//! 尚未返回的请求上下文。

use super::PredictionKind;

pub(crate) struct PendingPrediction {
    pub(super) kind: PredictionKind,
    pub(super) scope: String,
    pub(super) question_guess: String,
}
