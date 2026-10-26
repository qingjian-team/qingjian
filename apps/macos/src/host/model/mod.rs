//! 本地整句模型：后台加载、停顿后请求重排、结果到了重画当前页。
//!
//! 按键回调里永远只跑词级模型；模型的意见在停键 80 毫秒后请求、二三十毫秒后到，只换候选窗口里的整句候选，
//! 用户翻过页或动过高亮就不打扰。前文优先用应用里光标前的文字（`refresh` 每次查询前给 Engine），应用给不出退回本会话历史。

use std::sync::Arc;
use std::sync::mpsc::{TryRecvError, channel};

use qingjian_core::sentence::SentenceScorer;
use qingjian_neural::zhiyi::ChoiceScorer;
use qingjian_neural::{CharScorer, NeuralError, P2cScorer};
use qingjian_platform::LocalModel;

mod loaded;
mod rescore_monitor;

pub use loaded::LoadedModel;
pub(super) use rescore_monitor::RescoreMonitor;

use super::*;

impl Host {
    /// 在后台线程加载选的模型并预热（第一次前向要编译 Metal 内核，几百毫秒），加载完由 [`Self::attach_loaded_model`] 接上。
    /// 选的知微、知意还没下载就用通变；一个模型文件都没有就什么都不做。
    pub(super) fn load_local_model(&mut self, wanted: LocalModel) {
        if self.model_loader.is_some()
            || self.engine.has_sentence_scorer()
            || self.engine.has_choice_model()
        {
            return;
        }
        if wanted == LocalModel::Zhiyi {
            match paths::zhiyi_model_dir() {
                Some(dir) => return self.load_zhiyi_model(dir),
                None => tracing::warn!("选了知意但没下载，改用含章·通变"),
            }
        }
        let zhiwei = (wanted == LocalModel::Zhiwei)
            .then(paths::user_zhiwei_model_path)
            .flatten();
        if wanted == LocalModel::Zhiwei && zhiwei.is_none() {
            tracing::warn!("选了含章·知微但没下载，改用含章·通变");
        }
        // 没选知微时通变优先：冻结集 8322 句上比字级模型的重排高 0.69 个点、延迟还低（docs/notes/model-eval.md）
        let (path, p2c) = match zhiwei {
            Some(path) => (path, false),
            None => match paths::p2c_model_path() {
                Some(path) => (path, true),
                None => match paths::model_path() {
                    Some(path) => (path, false),
                    None => {
                        tracing::info!("没有本地模型文件，不重排");
                        return;
                    }
                },
            },
        };
        let (tx, rx) = channel::<Result<LoadedModel, NeuralError>>();
        let spawned = std::thread::Builder::new()
            .name("qingjian-model-load".to_owned())
            .spawn(move || {
                let started = std::time::Instant::now();
                // 预热要走各自真正的前向：第一次前向要编译 Metal 内核，几百毫秒
                let loaded = CharScorer::load(&path).and_then(|scorer| {
                    if p2c {
                        let scorer = P2cScorer::new(scorer)
                            .ok_or(NeuralError::Corrupt("含章·通变模型缺少 <sep> 分隔符"))?;
                        scorer.0.score_p2c("ni", &["你"])?;
                        Ok(Box::new(scorer) as Box<dyn SentenceScorer>)
                    } else {
                        scorer.score("", &["的"])?;
                        Ok(Box::new(scorer) as Box<dyn SentenceScorer>)
                    }
                });
                if loaded.is_ok() {
                    tracing::info!(
                        path = %path.display(),
                        kind = if p2c { "含章·通变" } else { "含章·知微" },
                        total_ms = started.elapsed().as_millis(),
                        "本地模型已加载并预热"
                    );
                }
                let _ = tx.send(loaded.map(LoadedModel::Sentence));
            });
        self.watch_model_loading(spawned.map(|_| rx));
    }

    /// 知意选词模型（试验）：加载时把线性层量化成 Q8_0，要一两秒。
    fn load_zhiyi_model(&mut self, dir: std::path::PathBuf) {
        let (tx, rx) = channel::<Result<LoadedModel, NeuralError>>();
        let spawned = std::thread::Builder::new()
            .name("qingjian-model-load".to_owned())
            .spawn(move || {
                let started = std::time::Instant::now();
                let loaded = ChoiceScorer::load(&dir).and_then(|scorer| {
                    let typed = scorer.letter_units("ni");
                    scorer.score("", &typed, &["ni", "nin"])?;
                    Ok(LoadedModel::Choice(Arc::new(scorer)))
                });
                if loaded.is_ok() {
                    tracing::info!(
                        path = %dir.display(),
                        total_ms = started.elapsed().as_millis(),
                        "知意选词模型已加载并预热"
                    );
                }
                let _ = tx.send(loaded);
            });
        self.watch_model_loading(spawned.map(|_| rx));
    }

    fn watch_model_loading(
        &mut self,
        spawned: std::io::Result<std::sync::mpsc::Receiver<Result<LoadedModel, NeuralError>>>,
    ) {
        match spawned {
            Ok(rx) => {
                self.model_loader = Some(rx);
                self.rescore.watch_loading();
            }
            Err(error) => tracing::warn!(%error, "起不了模型加载线程，本地整句模型不用"),
        }
    }

    /// 加载线程有结果了就接到 Engine 上；每次查询和加载定时器都会看一眼，不阻塞。
    pub fn attach_loaded_model(&mut self) {
        let Some(rx) = &self.model_loader else {
            self.rescore.stop_watching();
            return;
        };
        match rx.try_recv() {
            Ok(Ok(loaded)) => {
                match loaded {
                    LoadedModel::Sentence(scorer) => {
                        self.engine.set_async_sentence_scorer(Some(scorer))
                    }
                    LoadedModel::Choice(model) => self.engine.set_choice_model(Some(model)),
                }
                self.model_loader = None;
                self.rescore.stop_watching();
                // 模型上线了：日志里补一条会话信息，之后的条目知道重排开着
                let version = self.version.clone();
                self.engine.log_session(&version, "macos");
                self.rescore_current_round();
            }
            Ok(Err(error)) => {
                tracing::warn!(%error, "本地模型加载失败，不重排");
                self.model_loader = None;
                self.rescore.stop_watching();
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.model_loader = None;
                self.rescore.stop_watching();
            }
        }
    }

    /// 模型接上时用户正在组句：这一轮的查询从没见过打分器，不补查一次就永远错过重排。
    /// 补查只为攒下整句路径、起防抖，不动画面；用户已翻页或动过高亮就不打扰。
    fn rescore_current_round(&mut self) {
        if self.engine.composition().is_empty()
            || self.translation.is_some()
            || self.session.page != 0
            || self.session.navigated
        {
            return;
        }
        if self.engine.query().is_ok() {
            tracing::info!("模型接上时正在组句，补一轮重排");
            self.schedule_rescoring();
        }
    }

    /// 模型还在后台加载。
    pub fn model_loading(&self) -> bool {
        self.model_loader.is_some()
    }

    /// 卸掉模型（配置关掉）。
    pub(super) fn unload_local_model(&mut self) {
        self.model_loader = None;
        self.engine.set_async_sentence_scorer(None);
        self.engine.set_choice_model(None);
        self.rescore.stop_watching();
        self.rescore.stop();
    }

    /// 每次查询之后：有整句路径等着打分就起防抖计时。
    pub fn schedule_rescoring(&mut self) {
        if self.engine.rescoring_pending() {
            self.rescore.schedule();
        }
    }

    /// 防抖到点：把攒着的整句路径送去后台，开始轮询。
    pub fn start_rescoring(&mut self) {
        if self.engine.composition().is_empty() {
            self.rescore.stop();
            return;
        }
        if self.engine.request_rescoring() {
            self.rescore.start_polling();
        }
    }

    /// 轮询到点：分回来了就重查一次、重画当前页；用户已翻页或动过高亮就只留着分不动画面。
    pub fn poll_rescoring(&mut self) {
        if self.engine.composition().is_empty() || self.translation.is_some() {
            self.rescore.stop();
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
        // 符号面板开着时格子里是面板条目，不拿重排结果换掉
        if self.session.page != 0 || self.session.navigated || self.engine.symbol_panel().is_some()
        {
            return;
        }
        let Ok(mut query) = self.engine.query() else {
            return;
        };
        self.engine.annotate(&mut query.candidates);
        let preedit = Preedit::from_marked(&query.marked_segments(), query.segments_cursor());
        let cloud = self.session.layout.cloud().to_vec();
        let candidates = std::mem::take(&mut query.candidates.items);
        self.reset_session(preedit, candidates);
        self.session.spelling = Some(query);
        // 高亮回到首选，切法不变，应用里的 marked text 不用动；只换窗口拼音行（带调拼音）
        self.follow_highlight();
        if !cloud.is_empty() {
            self.session.layout.set_cloud(cloud);
        }
        self.render();
        // 结果到的时候用户可能又敲了键、新的防抖还没到点，上面的 stop() 把它一起清了；
        // 这次查询要是又记下了要打分 / 生成的，再起一轮，否则最后这段按键永远不请求（Linux 的 requery_rescored 同理）
        self.schedule_rescoring();
    }
}
