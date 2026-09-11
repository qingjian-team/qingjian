//! 本地整句模型（与 macOS 壳的 `host/model.rs` 对齐）：后台加载、停键后请求重排、结果到了重画当前页。
//!
//! 按键回调里永远只跑词级模型；模型的意见在停键 80 毫秒后请求、几十毫秒后到，只换候选窗口里的整句候选，
//! 用户翻过页或动过高亮就不打扰。前文优先用应用里光标前的文字（DLL 起组句时随 `ClientMessage::Surrounding` 送来），没有退回本会话历史。
//! 节拍由工人循环驱动：[`Router::next_tick`] 说下次多久来一次 [`Router::tick`]；DLL 组句期间每 80 ms 的 `Poll` 也顺带 tick。
//! 加载在 [`loader`]，进行态在 [`state`]。

mod loader;
mod state;

use std::path::{Path, PathBuf};
use std::time::Duration;

use qingjian_core::CandidateLayout;
use qingjian_platform::LocalModelConfig;
use qingjian_platform::protocol::SessionId;

use self::loader::Loaded;
pub(crate) use self::loader::ModelLoader;
pub(crate) use self::state::RescoreState;
use super::Router;
use super::composed::Composed;

/// 找模型目录：用户目录 `model/` 优先（用户自己的模型），否则随包 `data/model/`；都没有三件套为 `None`。
pub fn find_model_dir(user_dir: Option<&Path>, bundled_root: &Path) -> Option<PathBuf> {
    let candidates = [
        user_dir.map(|dir| dir.join("model")),
        Some(bundled_root.join("data/model")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|dir| dir.join("model.safetensors").is_file())
}

impl Router {
    /// 启动时：记下模型目录，按 `[model] enabled` 决定要不要加载。
    pub fn configure_local_model(&mut self, model_dir: Option<PathBuf>, config: &LocalModelConfig) {
        self.model_dir = model_dir;
        self.applied_model = config.clone();
        if config.enabled {
            self.load_local_model();
        }
    }

    /// 热加载：`[model]` 变了才重载 / 卸载。
    pub(super) fn apply_model_config(&mut self, config: &LocalModelConfig) {
        if *config == self.applied_model {
            return;
        }
        self.applied_model = config.clone();
        if config.enabled {
            self.load_local_model();
        } else {
            self.unload_local_model();
        }
    }

    /// 在后台线程加载模型；没有模型文件就什么都不做。
    fn load_local_model(&mut self) {
        if self.model_loader.is_some() || self.engine.has_sentence_scorer() {
            return;
        }
        let Some(dir) = &self.model_dir else {
            tracing::info!("没有本地整句模型文件，不重排");
            return;
        };
        self.model_loader = ModelLoader::spawn(dir);
    }

    /// 卸掉模型（配置关掉）。
    fn unload_local_model(&mut self) {
        self.model_loader = None;
        self.engine.set_async_sentence_scorer(None);
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
                match *result {
                    Ok(scorer) => self
                        .engine
                        .set_async_sentence_scorer(Some(Box::new(scorer))),
                    Err(error) => tracing::warn!(%error, "本地整句模型加载失败，不重排"),
                }
                self.model_loader = None;
            }
            Loaded::Gone => self.model_loader = None,
        }
    }

    /// 组句结束：什么都不等了；应用前文也作废（下一段组句 DLL 会再送）。
    pub(super) fn stop_rescoring(&mut self) {
        self.rescore.stop();
        self.engine.set_rescoring_context(None);
    }

    /// DLL 送来聚焦会话的光标前文：给 Engine 当前文，缓存里按旧前文记的「要打分的」作废，重新攒一次并重新计时。
    /// 组句已经结束 / 不是聚焦会话的丢掉。
    pub(super) fn set_surrounding(&mut self, session: SessionId, text: String) {
        if self.focused != Some(session) || self.engine.composition().is_empty() {
            return;
        }
        self.engine
            .set_rescoring_context((!text.is_empty()).then_some(text));
        if matches!(self.composed, Some(Composed::Candidates { .. })) {
            // 查一次只为按新前文重新记下要打分的文本，候选顺序此刻不变
            let _ = self.engine.query();
            self.schedule_rescoring();
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

    /// 工人循环下次该多久后来一次 [`Self::tick`]：在等重排就按它的节拍，否则按配置热加载的一秒。
    pub fn next_tick(&self) -> Duration {
        self.rescore
            .next_deadline()
            .map_or(super::reload::CONFIG_POLL_INTERVAL, |deadline| {
                deadline.min(super::reload::CONFIG_POLL_INTERVAL)
            })
    }

    /// 到点了：接上加载好的模型、推进重排、看一眼配置文件。工人循环超时与 DLL 的 `Poll` 都会调。
    pub fn tick(&mut self) {
        self.attach_loaded_model();
        self.advance_rescoring();
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
        let Ok(query) = self.engine.query() else {
            return;
        };
        let Some(Composed::Candidates {
            preedit,
            cursor,
            layout,
        }) = self.composed.as_mut()
        else {
            return;
        };
        let cloud = layout.cloud().to_vec();
        let mut rebuilt = CandidateLayout::new(
            query.candidates.items.clone(),
            self.config.page_size,
            self.config.cloud_slots,
        );
        if !cloud.is_empty() {
            rebuilt.set_cloud(cloud);
        }
        *layout = rebuilt;
        *preedit = query.marked_segments().iter().map(Into::into).collect();
        *cursor = query.marked_cursor();
        let frame = self.current_frame();
        self.reconcile_candidates(&frame);
        // 前文在结果回来之前换过（Surrounding 晚到）：这次查询又记下了一批要打分的，再来一轮
        self.schedule_rescoring();
    }
}
