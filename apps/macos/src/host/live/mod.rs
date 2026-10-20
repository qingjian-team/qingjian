//! 本地实时翻译的会话接线与独立显示；断句和网络请求由 predict 负责。

mod display;
mod monitor;
mod session;
mod ticker;

pub use session::LiveSession;

use crate::candidates::{Frame, Row};
use crate::host::Host;

impl Host {
    pub fn reset_live_translation(&mut self) {
        self.live.reset();
    }

    /// 只记录实际交给应用的文本，不能把候选预览再次当作上屏内容。
    pub fn commit_live_text(&mut self, text: &str) {
        if !self.live.active() {
            return;
        }
        self.live.tracker.commit(text);
        self.submit_live_translation();
    }

    pub fn backspace_live_text(&mut self) {
        if self.live.active() {
            self.live.tracker.backspace();
            self.submit_live_translation();
        }
    }

    /// 导航与查询都会走这里，因此高亮候选变化也立即使旧请求失效。
    pub fn preview_live_translation(&mut self) {
        if !self.live.active() || self.translation.is_some() {
            return;
        }
        let preview = if self.engine.composition().is_empty() {
            String::new()
        } else {
            self.session
                .candidate(self.session.highlighted)
                .map(|candidate| candidate.text)
                .unwrap_or_else(|| self.engine.composition().text().to_owned())
        };
        self.live.tracker.preview(&preview);
        self.submit_live_translation();
    }

    fn submit_live_translation(&mut self) {
        let source = self.live.tracker.source();
        if source == self.live.source {
            self.render_live_translation();
            return;
        }
        self.live.source = source.clone();
        self.live.display.begin(self.live.unavailable.as_deref());
        if source.trim().is_empty() {
            if let Some(translator) = &mut self.live.translator {
                translator.cancel();
            }
            self.live.monitor.stop();
            self.live.display.reset();
            self.live.window.hide();
            return;
        }
        if let Some(translator) = &mut self.live.translator {
            translator.submit(source);
            self.live.monitor.start();
        }
        self.render_live_translation();
    }

    pub fn poll_live_translation(&mut self) {
        if crate::imk::secure_input::enabled() {
            self.live.reset();
            return;
        }
        let result = self
            .live
            .translator
            .as_mut()
            .and_then(|worker| worker.poll());
        if let Some(result) = result
            && result.source == self.live.source
        {
            self.live.display.complete(result.result);
            self.live.monitor.stop();
            self.render_live_translation();
        }
    }

    fn render_live_translation(&mut self) {
        if self.live.source.trim().is_empty() || !self.live.active() {
            self.live.window.hide();
            return;
        }
        let (text, status) = self.live.display.presentation();
        let rows = qingjian_render::translation_lines(text, 48, 8)
            .into_iter()
            .map(|text| Row {
                index: String::new(),
                text,
                annotation: Vec::new(),
                cloud: false,
            })
            .collect();
        self.live.window.show_translation(
            Frame {
                rows,
                highlighted: usize::MAX,
                footer: Some(status.to_owned()),
                ..Frame::default()
            },
            self.anchor,
            self.window.visible_frame(),
        );
    }
}
