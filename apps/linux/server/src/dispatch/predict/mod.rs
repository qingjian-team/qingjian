//! 云联想：按 `[predict]` 把云端预测器接到 Engine 上，组句时发一次请求，结果到了把云端词并入第一页末尾、
//! 整句补全留给 Tab。与 Windows Server 的 `dispatch/reload::attach_cloud` 和 `dispatch/composed` 云端分支对齐。
//!
//! 防抖、超时和缓存都在 `qingjian_predict::Worker` 自己的线程里，Server 只等结果，节拍见 [`state`]。
//! 首版不读应用光标前后的文字（Fcitx5 的 surrounding text 各家实现不一致），只按拼音与本地候选联想；
//! 关着时 `[predict] slots` 的格子不存在，不画占位。Linux 没有配置热加载，改完 `[predict]` 要重启服务。

mod state;

#[cfg(test)]
mod tests;

use qingjian_core::{Candidate, CloudWord, NoGlossFiller, NoPredictor};
use qingjian_predict::{CloudGlossFiller, CloudPredictor, PredictConfig};

pub(crate) use self::state::PredictState;
use super::Router;
use super::composed::Composed;

impl Router {
    /// 接云联想与释义兜底；关着或缺密钥退回本地实现。
    pub fn configure_cloud(&mut self, predict: &PredictConfig) {
        if !predict.enabled {
            tracing::info!("云联想未开启（[predict] enabled = false）");
            self.engine.set_predictor(Box::new(NoPredictor));
            self.engine.set_gloss_filler(Box::new(NoGlossFiller));
            return;
        }
        match CloudPredictor::new(predict) {
            Ok(predictor) => {
                tracing::info!(
                    model = %predict.model,
                    slots = self.config.cloud_slots,
                    "云联想已接入"
                );
                self.engine.set_predictor(Box::new(predictor));
            }
            Err(error) => {
                tracing::warn!(%error, "云联想接入失败（缺 API key？），退回本地候选");
                self.engine.set_predictor(Box::new(NoPredictor));
            }
        }
        match CloudGlossFiller::new(predict) {
            Ok(filler) => self.engine.set_gloss_filler(Box::new(filler)),
            Err(error) => {
                tracing::warn!(%error, "释义兜底未启用");
                self.engine.set_gloss_filler(Box::new(NoGlossFiller));
            }
        }
    }

    /// 缓冲变化后发一次请求，`local` 是当前的本地候选（云端只取前几个当提示）。
    /// 私密输入、辅码筛选和拼音太短由 Engine 自己挡下，这里不重复判断。
    pub(super) fn request_prediction(&mut self, local: &[Candidate]) {
        if !self.engine.prediction_enabled() {
            return;
        }
        if self.engine.request_prediction(None, local).is_some() {
            self.predict.start_polling();
        } else {
            self.predict.stop();
        }
    }

    /// 组句结束、换会话、转私密输入：不等结果了。迟到的结果按序号作废，不会画到下一轮上。
    pub(super) fn cancel_prediction(&mut self) {
        self.predict.stop();
        if self.engine.prediction_enabled() {
            self.engine.cancel_prediction();
        }
    }

    /// 到点了收一次结果：云端词并进当前布局，整句补全记下；等太久就什么都不显示。
    pub(super) fn advance_prediction(&mut self) {
        if !self.predict.polling() {
            return;
        }
        if self.engine.composition().is_empty() {
            self.cancel_prediction();
            return;
        }
        let Some(prediction) = self.engine.poll_prediction() else {
            if self.predict.expired() {
                tracing::debug!("等云联想超时，本轮不显示云端词");
                self.predict.stop();
            }
            return;
        };
        self.predict.stop();
        if let Some(Composed::Candidates { layout, .. }) = self.composed.as_mut() {
            let words: Vec<Candidate> = prediction
                .words
                .into_iter()
                .map(CloudWord::into_candidate)
                .collect();
            layout.set_cloud(words);
            self.sentence = prediction.sentence;
        }
    }
}
