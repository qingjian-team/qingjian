//! 接上的选词模型与它的缓存。

use std::cell::RefCell;
use std::sync::Arc;

use super::worker::ChoiceWorker;
use super::{ChoiceModel, ChoiceRequest};

/// 接上的选词模型：同步对齐用的模型、后台线程，和最近一页的分。
pub(crate) struct ChoiceRanker {
    pub model: Arc<dyn ChoiceModel>,
    pub worker: ChoiceWorker,

    /// 最近打好分的一页；候选窗里一次只有一页要排，留一条就够。
    pub scored: RefCell<Option<(ChoiceRequest, Vec<f32>, u64)>>,

    /// 查询时还没分、等壳在停顿后送去后台的那页。
    pub wanted: RefCell<Option<ChoiceRequest>>,
}

impl ChoiceRanker {
    pub fn new(model: Arc<dyn ChoiceModel>) -> Self {
        Self {
            worker: ChoiceWorker::spawn(model.clone()),
            model,
            scored: RefCell::new(None),
            wanted: RefCell::new(None),
        }
    }

    /// 这页的分与打分花的毫秒数。
    pub fn cached(&self, request: &ChoiceRequest) -> Option<(Vec<f32>, u64)> {
        self.scored
            .borrow()
            .as_ref()
            .filter(|(scored, _, _)| scored == request)
            .map(|(_, logits, ms)| (logits.clone(), *ms))
    }
}
