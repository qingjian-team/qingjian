use super::policy::PredictionPolicy;
use super::request::PredictionRequest;
use super::response::Prediction;

/// 联想提供方。实现可以联网，但接口是**非阻塞**的：提交立刻返回，结果由壳定时轮询。
///
/// 「最新请求优先」：实现收到新请求时可以直接丢掉还没发出去的旧请求；
/// Engine 只认序号等于最近一次提交的结果，其余一律丢弃。
pub trait Predictor: Send {
    /// 观察窗口、条数上限与开关。
    fn policy(&self) -> PredictionPolicy;

    /// 提交请求，不能阻塞。
    fn submit(&mut self, request: PredictionRequest);

    /// 取一条已到达的结果，没有就返回 `None`，不能阻塞。
    fn poll(&mut self) -> Option<Prediction>;

    /// 是否真的会联想；`false` 时 Engine 根本不构造请求。
    fn is_enabled(&self) -> bool {
        true
    }
}

/// 不联想。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPredictor;

impl Predictor for NoPredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy::default()
    }

    fn submit(&mut self, _request: PredictionRequest) {}

    fn poll(&mut self) -> Option<Prediction> {
        None
    }

    fn is_enabled(&self) -> bool {
        false
    }
}
