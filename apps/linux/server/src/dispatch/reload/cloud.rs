//! 更换云服务时撤销旧请求与翻译评审，并清理所有会话中的旧云候选。

use std::path::PathBuf;

use qingjian_core::CandidateLayout;
use qingjian_predict::PredictConfig;

use crate::assembly;
use crate::dispatch::{Router, composed::Composed};

fn clear_cloud(composed: &mut Option<Composed>, highlight: &mut usize, page: usize, slots: usize) {
    if let Some(Composed::Candidates { layout, .. }) = composed {
        let selected = layout.candidate(*highlight).cloned();
        *layout = CandidateLayout::new(layout.local().to_vec(), page, slots);
        *highlight = (0..layout.len())
            .find(|&index| {
                layout.candidate(index).is_some() && layout.candidate(index) == selected.as_ref()
            })
            .unwrap_or(0);
    }
}

impl Router {
    pub(super) fn reload_cloud(&mut self, predict: &PredictConfig, usage_path: Option<PathBuf>) {
        self.end_translation();
        assembly::attach_cloud(&mut self.engine, predict, usage_path);
        if !self.engine.prediction_enabled() {
            self.config.cloud_slots = 0;
        }
        self.sentence = None;
        self.engine.note_displayed(std::iter::empty());
        clear_cloud(
            &mut self.composed,
            &mut self.highlight,
            self.config.page_size,
            self.config.cloud_slots,
        );
        for session in self.sessions.values_mut() {
            clear_cloud(
                &mut session.composed,
                &mut session.highlight,
                self.config.page_size,
                self.config.cloud_slots,
            );
            session.display_frame = None;
            session.last_frame = None;
        }
        self.request_current_prediction();
    }
}
