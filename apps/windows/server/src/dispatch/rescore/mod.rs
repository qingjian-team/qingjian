//! 本地整句模型（与 macOS 壳的 `host/model.rs` 对齐）：后台加载、停键后请求重排、结果到了重画当前页。
//!
//! 按键回调里永远只跑词级模型；模型的意见在停键 80 毫秒后请求、几十毫秒后到，只换候选窗口里的整句候选，
//! 用户翻过页或动过高亮就不打扰。前文优先用应用里光标前的文字（DLL 起组句时随 `ClientMessage::Surrounding` 送来），没有退回本会话历史。
//! 节拍由工人循环驱动：[`Router::next_tick`] 说下次多久来一次 [`Router::tick`]；DLL 组句期间每 80 ms 的 `Poll` 也顺带 tick。
//! 找模型在 [`models`]，加载在 [`loader`]，进行态在 [`state`]。

mod loader;
mod models;
mod state;

use std::time::Duration;

use qingjian_core::CandidateLayout;
use qingjian_platform::LocalModelConfig;
use qingjian_platform::protocol::SessionId;

pub(crate) use self::loader::ModelLoader;
use self::loader::{Loaded, LoadedModel};
pub use self::models::{ModelDirs, find_model};
pub(crate) use self::state::RescoreState;
use super::Router;
use super::composed::{Composed, marked_parts};

impl Router {
    /// 启动时：记下去哪找模型，按 `[model] enabled` 决定要不要加载。
    pub fn configure_local_model(&mut self, dirs: ModelDirs, config: &LocalModelConfig) {
        self.model_dirs = dirs;
        self.applied_model = config.clone();
        self.apply_choice_tier();
        if config.enabled {
            self.load_local_model();
        }
    }

    /// 热加载：开关或选的模型变了才重载 / 卸载，档位每次都套。
    pub(super) fn apply_model_config(&mut self, config: &LocalModelConfig) {
        let changed =
            (config.enabled, config.name) != (self.applied_model.enabled, self.applied_model.name);
        self.applied_model = config.clone();
        self.apply_choice_tier();
        if !changed {
            return;
        }
        self.unload_local_model();
        if config.enabled {
            self.load_local_model();
        }
    }

    fn apply_choice_tier(&mut self) {
        self.engine
            .set_choice_tier(self.applied_model.zhiyi_tier.choice_tier());
    }

    /// 在后台线程加载选的模型；没有模型文件就什么都不做。
    fn load_local_model(&mut self) {
        if self.model_loader.is_some()
            || self.engine.has_sentence_scorer()
            || self.engine.has_choice_model()
        {
            return;
        }
        let Some(files) = self.model_dirs.resolve(self.applied_model.name) else {
            tracing::info!("没有本地模型文件，不重排");
            return;
        };
        self.model_loader = ModelLoader::spawn(files);
    }

    /// 卸掉模型（配置关掉或换了模型）。
    fn unload_local_model(&mut self) {
        self.model_loader = None;
        self.engine.set_async_sentence_scorer(None);
        self.engine.set_choice_model(None);
        self.rescore.stop();
    }

    /// 加载线程有结果了就接到 Engine 上；每次按键 / tick 顺手看一眼，不阻塞。
    pub(super) fn attach_loaded_model(&mut self) {
        let Some(loader) = &self.model_loader else {
            return;
        };
        match loader.poll() {
            Loaded::Pending => {}
            Loaded::Done(result) => {
                match result {
                    Ok(loaded) => {
                        match loaded {
                            LoadedModel::Sentence(scorer) => {
                                self.engine.set_async_sentence_scorer(Some(scorer));
                            }
                            LoadedModel::Choice(model) => {
                                self.engine.set_choice_model(Some(model));
                            }
                        }
                        // 模型上线了：日志里补一条会话信息，之后的条目知道重排开着
                        self.engine
                            .log_session(env!("CARGO_PKG_VERSION"), "windows");
                    }
                    Err(error) => tracing::warn!(%error, "本地模型加载失败，不重排"),
                }
                self.model_loader = None;
            }
            Loaded::Gone => self.model_loader = None,
        }
    }

    /// 组句结束：什么都不等了；应用前文也作废（下一段组句 DLL 会再送）。
    pub(super) fn stop_rescoring(&mut self) {
        self.rescore.stop();
        self.surrounding_before.clear();
        self.engine.set_rescoring_context(None);
    }

    /// DLL 送来聚焦会话的光标前文：给 Engine 当前文，缓存里按旧前文记的「要打分的」作废，重新攒一次并重新计时。
    /// 组句已经结束 / 不是聚焦会话的丢掉。
    pub(super) fn set_surrounding(&mut self, session: SessionId, text: String) {
        if self.focused != Some(session) || self.engine.composition().is_empty() {
            return;
        }
        self.surrounding_before = text.clone();
        self.engine
            .set_rescoring_context((!text.is_empty()).then_some(text));
        if matches!(self.composed, Some(Composed::Candidates { .. })) {
            // 查一次只为按新前文重新记下要打分的文本，候选顺序此刻不变
            let _ = self.engine.query();
            self.schedule_rescoring();
            // 云联想也在等这份前文：带着新上下文重发，作废在飞的空上下文请求（worker 端有防抖）
            self.refresh_prediction();
        }
    }

    /// 缓冲变化之后：有整句路径等着打分就起防抖计时，否则停下。
    pub(super) fn schedule_rescoring(&mut self) {
        if self.engine.rescoring_pending() {
            self.rescore.schedule();
        } else {
            self.rescore.stop();
        }
    }

    /// 工人循环下次该多久后来一次 [`Self::tick`]：在等重排、键位提示在等停顿就按它们的节拍，否则按配置热加载的一秒。
    pub fn next_tick(&self) -> Duration {
        [self.rescore.next_deadline(), self.key_hint.next_deadline()]
            .into_iter()
            .flatten()
            .fold(super::reload::CONFIG_POLL_INTERVAL, Duration::min)
    }

    /// 到点了：接上加载好的模型、推进重排、键位提示停顿到点就弹出、看一眼配置文件。工人循环超时与 DLL 的 `Poll` 都会调。
    pub fn tick(&mut self) {
        self.attach_loaded_model();
        self.advance_rescoring();
        self.advance_key_hint();
        self.poll_config_reload();
    }

    /// 防抖到点就发请求；在等结果就收一次，收到了重查并重画当前页。
    fn advance_rescoring(&mut self) {
        if self.engine.composition().is_empty() || self.translation.is_some() {
            self.rescore.stop();
            return;
        }
        if self.rescore.debounce_elapsed() {
            if self.engine.request_rescoring() {
                self.rescore.start_polling();
            } else {
                self.rescore.stop();
            }
        }
        if !self.rescore.polling() {
            return;
        }
        if !self.engine.poll_rescoring() {
            // 等太久多半是前文变了（上屏后接着打下一段）、结果作废；真卡住也只是这轮不重排
            if self.rescore.expired() {
                tracing::debug!("等本地整句模型超时，本轮不重排");
                self.rescore.stop();
            }
            return;
        }
        self.rescore.stop();
        if self.highlight >= self.config.page_size || self.navigated {
            return;
        }
        self.requery_rescored();
    }

    /// 分回来了：按重排后的顺序重建候选布局，云端词与整句补全留着，重画当前页。
    fn requery_rescored(&mut self) {
        let Ok(mut query) = self.engine.query() else {
            return;
        };
        let Some(Composed::Candidates {
            preedit,
            cursor,
            typed_keys,
            layout,
            spelling,
            ..
        }) = self.composed.as_mut()
        else {
            return;
        };
        let cloud = layout.cloud().to_vec();
        let mut rebuilt = CandidateLayout::new(
            std::mem::take(&mut query.candidates.items),
            self.config.page_size,
            self.config.cloud_slots,
        );
        if !cloud.is_empty() {
            if layout.cloud_first() {
                rebuilt.set_cloud_first(cloud);
            } else {
                rebuilt.set_cloud(cloud);
            }
        }
        *layout = rebuilt;
        (*preedit, *cursor, *typed_keys) = marked_parts(&query);
        **spelling = query;
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
        // 前文在结果回来之前换过（Surrounding 晚到）：这次查询又记下了一批要打分的，再来一轮
        self.schedule_rescoring();
    }
}
