//! 联想：候选之外的异步补充。
//!
//! [`Predictor`] 由壳注入（网络实现在 `qingjian-predict`，Core 永远不联网），Engine 负责裁剪上下文、
//! 编号请求、校验云端词的拼音、丢弃过期结果。联想**不参与排序、不阻塞输入**；
//! 云端词到了只补进候选窗口第一页末尾几格（[`crate::CandidateLayout`]），前面的本地候选不挪。

mod cloud_word;
mod fuzzy;
mod kind;
mod policy;
mod predictor;
mod question;
mod request;
mod response;
mod script;
mod surrounding_text;

pub use cloud_word::CloudWord;
pub use fuzzy::{mismatch_count, tolerance};
pub use kind::PredictionKind;
pub use policy::PredictionPolicy;
pub use predictor::{NoPredictor, Predictor};
pub use question::restates_question;
pub use request::PredictionRequest;
pub use response::Prediction;
pub use script::translation_target;
pub use surrounding_text::SurroundingText;
