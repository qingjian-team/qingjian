//! 组句中的云联想：发请求、收结果并进候选布局，按需联想与光标前文。

use qingjian_core::{CloudWord, SurroundingText};

use super::Composed;
use crate::dispatch::Router;

impl Router {
    /// 拉一次云联想结果：云端词并进候选布局，整句补全记下；翻译评审时结果是译文。
    pub(in crate::dispatch) fn poll_prediction(&mut self) {
        if !self.engine.prediction_enabled() {
            return;
        }
        let Some(prediction) = self.engine.poll_prediction() else {
            return;
        };
        if self
            .candidate_translation
            .as_ref()
            .is_some_and(|(sequence, _, _)| *sequence == prediction.sequence)
        {
            let (_, candidate, independent_language) =
                self.candidate_translation.take().expect("刚检查过候选译文");
            let generated = prediction.sentence.filter(|text| !text.trim().is_empty());
            let updated = if independent_language.is_some() {
                generated.is_some()
            } else {
                generated.as_ref().is_some_and(|generated| {
                    self.engine
                        .set_generated_candidate_translation(&candidate, generated)
                })
            };
            self.notice = Some(if updated {
                if let Some(language) = independent_language {
                    self.set_independent_candidate_translation(
                        &candidate.text,
                        language,
                        generated.as_deref().expect("updated 已检查"),
                    );
                    format!("已更新「{}」的中文译词", candidate.text)
                } else {
                    self.refresh_candidate_translation(&candidate.text);
                    format!("已更新「{}」的译文", candidate.text)
                }
            } else {
                "模型没有生成可用译文".to_owned()
            });
            return;
        }
        if self.translation.is_some() {
            match prediction.sentence {
                Some(text) => {
                    if let Some(translation) = self.translation.as_mut() {
                        translation.result = Some(text);
                    }
                }
                None => {
                    tracing::info!("翻译选中文字：云端没有给出译文");
                    self.translation = None;
                }
            }
            return;
        }
        if let Some(Composed::Candidates {
            layout,
            manual_sequence,
            ..
        }) = self.composed.as_mut()
        {
            let manual = manual_sequence.take() == Some(prediction.sequence);
            self.sentence = prediction.sentence;
            if self.navigated || self.highlight >= self.config.page_size {
                return;
            }
            let words = prediction
                .words
                .into_iter()
                .map(CloudWord::into_candidate)
                .collect();
            if manual {
                layout.set_cloud_first(words);
            } else {
                layout.set_cloud(words);
            }
        }
    }

    pub(in crate::dispatch) fn cancel_prediction(&mut self) {
        if let Some(Composed::Candidates {
            manual_sequence, ..
        }) = self.composed.as_mut()
        {
            *manual_sequence = None;
        }
        if self.engine.prediction_enabled() {
            self.engine.cancel_prediction();
        }
    }

    /// DLL 送来新的光标前文后重发一次云联想：作废在飞的空上下文请求，让云端拿得到 before。
    pub(in crate::dispatch) fn refresh_prediction(&mut self) {
        if !self.config.cloud_enabled {
            return;
        }
        let Some(Composed::Candidates {
            layout,
            manual_sequence,
            ..
        }) = &self.composed
        else {
            return;
        };
        if manual_sequence.is_some() {
            self.request_manual_prediction();
            return;
        }
        // 手动结果已到后保持当前布局；新输入会重建状态并恢复自动联想。
        if layout.cloud_first() {
            return;
        }
        let local = layout.local().to_vec();
        let surrounding = self.surrounding_for_prediction();
        self.engine.request_prediction(surrounding, &local);
    }

    /// 显式触发时使用同一份应用上下文，并记录序号供结果布局匹配。
    pub(in crate::dispatch) fn request_manual_prediction(&mut self) -> bool {
        if !self.config.cloud_enabled {
            return false;
        }
        let surrounding = self.surrounding_for_prediction();
        let Some(Composed::Candidates {
            layout,
            manual_sequence,
            ..
        }) = self.composed.as_mut()
        else {
            return false;
        };
        *manual_sequence = self
            .engine
            .request_manual_prediction(surrounding, layout.local());
        manual_sequence.is_some()
    }

    /// DLL 报来的光标前文包成云联想要的形状；没有就 `None`（Engine 只靠拼音猜）。
    pub(in crate::dispatch) fn surrounding_for_prediction(&self) -> Option<SurroundingText> {
        (!self.surrounding_before.is_empty()).then(|| SurroundingText {
            before: self.surrounding_before.clone(),
            after: String::new(),
        })
    }
}
