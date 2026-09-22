//! 本地整句模型（与 Windows Server 的 `dispatch/rescore` 对齐）：后台加载、停键后请求重排、结果到了重建当前布局。
//!
//! 按键回调里永远只跑词级模型；模型的意见在停键 80 毫秒后请求、几十毫秒后到，只换候选里的整句候选，
//! 用户翻过页或动过高亮就不打扰。前文用本会话最近上屏的字（首版不读应用里光标前的文字）。
//! 节拍由主循环驱动：[`Router::next_tick`] 说下次多久来一次 [`Router::tick`]；插件组句期间每 80 毫秒的 `Poll` 也顺带 tick。
//! 模型在后台接上时用户正在组句，这一轮也补一次重排。加载在 [`loader`]，进行态在 [`state`]。

mod loader;
mod state;

use std::path::{Path, PathBuf};
use std::time::Duration;

use qingjian_core::CandidateLayout;
use qingjian_core::sentence::SentenceScorer;
use qingjian_platform::LocalModelConfig;

use self::loader::Loaded;
pub(crate) use self::loader::ModelLoader;
pub(crate) use self::state::RescoreState;
use super::Router;
use super::composed::Composed;

/// 空闲时主循环多久醒一次：到点落盘学习数据。
const IDLE_TICK: Duration = Duration::from_secs(1);

/// 找模型（`.qjm` 单文件，或开发时的三件套目录）：用户目录 `model/` 优先（用户自己的模型），否则随包 `data/model/`；都没有为 `None`。
pub fn find_model(user_dir: Option<&Path>, bundled_root: &Path) -> Option<PathBuf> {
    let candidates = [
        user_dir.map(|dir| dir.join("model")),
        Some(bundled_root.join("data/model")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find_map(|dir| qingjian_neural::find_model(&dir))
}

impl Router {
    /// 启动时：记下模型文件，按 `[model] enabled` 决定要不要加载。Linux 没有配置热加载，关了要重启服务。
    pub fn configure_local_model(
        &mut self,
        model_path: Option<PathBuf>,
        config: &LocalModelConfig,
    ) {
        self.model_path = model_path;
        if config.enabled {
            self.load_local_model();
        } else {
            tracing::info!("本地整句模型已关（[model] enabled = false）");
        }
    }

    /// 在后台线程加载模型；没有模型文件就什么都不做。
    fn load_local_model(&mut self) {
        if self.model_loader.is_some() || self.engine.has_sentence_scorer() {
            return;
        }
        let Some(path) = &self.model_path else {
            tracing::info!("没有本地整句模型文件，不重排");
            return;
        };
        self.model_loader = ModelLoader::spawn(path);
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
                    Ok(scorer) => self.attach_sentence_scorer(Box::new(scorer)),
                    Err(error) => tracing::warn!(%error, "本地整句模型加载失败，不重排"),
                }
                self.model_loader = None;
            }
            Loaded::Gone => self.model_loader = None,
        }
    }

    /// 打分器接到 Engine 上（后台加载完成，或测试直接注入）：日志补一条会话信息，正在组句就补这一轮的重排。
    pub fn attach_sentence_scorer(&mut self, scorer: Box<dyn SentenceScorer>) {
        self.engine.set_async_sentence_scorer(Some(scorer));
        // 模型上线了：日志里补一条会话信息，之后的条目知道重排开着
        self.engine.log_session(env!("CARGO_PKG_VERSION"), "linux");
        self.rescore_current_round();
    }

    /// 模型接上时用户正在组句：这一轮的查询从没见过打分器，不补查一次就永远错过重排。
    /// 补查只为攒下整句路径、起防抖，不动画面；用户已翻页或动过高亮就不打扰。
    fn rescore_current_round(&mut self) {
        if !matches!(self.composed, Some(Composed::Candidates { .. })) || !self.on_first_page() {
            return;
        }
        if self.engine.query().is_ok() {
            tracing::info!("模型接上时正在组句，补一轮重排");
            self.schedule_rescoring();
        }
    }

    /// 组句结束 / 换会话：什么都不等了。
    pub(super) fn stop_rescoring(&mut self) {
        self.rescore.stop();
    }

    /// 缓冲变化之后：有整句路径等着打分就起防抖计时，否则停下。
    pub(super) fn schedule_rescoring(&mut self) {
        if self.engine.rescoring_pending() {
            self.rescore.schedule();
        } else {
            self.rescore.stop();
        }
    }

    /// 主循环下次该多久后来一次 [`Router::tick`]：在等重排就按它的节拍，否则按落盘学习的一秒。
    pub fn next_tick(&self) -> Duration {
        self.rescore
            .next_deadline()
            .map_or(IDLE_TICK, |deadline| deadline.min(IDLE_TICK))
    }

    /// 防抖到点就发请求；在等结果就收一次，收到了重查并重建当前布局。
    pub(super) fn advance_rescoring(&mut self) {
        if self.engine.composition().is_empty() {
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
        if self.on_first_page() {
            self.requery_rescored();
        }
    }

    /// 用户还看着第一页、没动过高亮：模型的结果才换正在看的这页。
    fn on_first_page(&self) -> bool {
        self.highlight < self.config.page_size && !self.navigated
    }

    /// 分回来了：按重排后的顺序重建候选布局，下一次 `Poll` 回的帧就是新顺序。
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
        *layout = CandidateLayout::new(
            query.candidates.items.clone(),
            self.config.page_size,
            self.config.cloud_slots,
        );
        *preedit = query.marked_segments().iter().map(Into::into).collect();
        *cursor = query.marked_cursor();
        // 这次查询可能又记下了一批要打分的（缓存按前文记，前文没变时不会），再来一轮
        self.schedule_rescoring();
    }
}
