//! 每个激活输入会话的本地翻译状态；与选区翻译、云联想独立。

use objc2::MainThreadMarker;
use qingjian_platform::{CandidateRenderer, LayoutMode};
use qingjian_predict::live::{LiveTranslationConfig, LiveTranslator, SentenceTracker};

use super::display::DisplayState;
use super::monitor::LiveMonitor;
use crate::candidates::CandidateWindow;

pub struct LiveSession {
    pub window: CandidateWindow,

    pub tracker: SentenceTracker,

    pub translator: Option<LiveTranslator>,

    pub monitor: LiveMonitor,

    pub source: String,

    pub(super) display: DisplayState,

    pub unavailable: Option<String>,

    pub epoch: u64,

    applied: Option<LiveTranslationConfig>,
}

impl LiveSession {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let window = CandidateWindow::new(mtm);
        window.set_layout(LayoutMode::Vertical);
        window.set_renderer(CandidateRenderer::Qingjian);
        Self {
            window,
            tracker: SentenceTracker::default(),
            translator: None,
            monitor: LiveMonitor::new(mtm),
            source: String::new(),
            display: DisplayState::default(),
            unavailable: None,
            epoch: 0,
            applied: None,
        }
    }

    pub fn active(&self) -> bool {
        self.applied.as_ref().is_some_and(|config| config.enabled)
            && !crate::imk::secure_input::enabled()
    }

    pub fn configure(&mut self, config: &LiveTranslationConfig, force: bool) {
        if !force && self.applied.as_ref() == Some(config) {
            return;
        }
        self.reset();
        self.unavailable = None;
        self.translator = if config.enabled {
            match LiveTranslator::new(config.clone()) {
                Ok(translator) => Some(translator),
                Err(error) => {
                    tracing::warn!(%error, "本地实时翻译未启用");
                    self.unavailable = Some(error);
                    None
                }
            }
        } else {
            None
        };
        self.applied = Some(config.clone());
    }

    pub fn reset(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.tracker.reset();
        if let Some(translator) = &mut self.translator {
            translator.cancel();
        }
        self.source.clear();
        self.display.reset();
        self.monitor.stop();
        self.window.hide();
    }
}
