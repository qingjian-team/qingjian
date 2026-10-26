//! 候选窗口输出：Router 只产出 [`Frame`]（符号面板开着时是 [`PanelFrame`](super::PanelFrame)）与光标矩形，
//! 交给 [`CandidateSink`] 去画；帧没变就不重画。

mod sink;

use qingjian_platform::protocol::{Frame, ScreenRect, SessionId};
use qingjian_render::Mode;

pub use self::sink::{CandidateMenu, CandidateMouse, CandidateSink, NoopSink, RenderSettings};
use super::Router;

impl Router {
    /// 空帧收窗口；非空且已知光标矩形就重绘；还没收到矩形（组句刚起）先不显示，免得在旧位置闪一下。
    /// 符号面板开着时画面板，候选窗收起。
    pub(super) fn reconcile_candidates(&mut self, frame: &Frame) {
        if let Some(panel) = self.panel_frame() {
            self.reconcile_panel(panel);
            return;
        }
        self.hide_panel_window();
        if frame.is_empty() {
            self.engine.note_displayed(std::iter::empty());
            // 组句结束：顺带清掉键位提示「卡住时显示」的状态
            self.key_hint_view();
            self.hide_candidate_window();
        } else if let Some(rect) = self.last_rect {
            let mode = self.input_mode();
            let key_hint = self.key_hint_view();
            let unchanged = matches!(&self.last_shown,
                Some((f, m, r, k)) if f == frame && *m == mode && *r == rect && *k == key_hint);
            if !unchanged {
                // 词汇记录的「看到轮次」按真正显示的页算，与 macOS 壳对齐。
                self.engine.note_displayed(frame.candidates.items.iter());
                self.candidates
                    .show(frame.clone(), mode.clone(), rect, key_hint.clone());
                self.last_shown = Some((frame.clone(), mode, rect, key_hint));
            }
        }
    }

    /// 给主题显示的输入状态，候选窗口与状态条共用。
    pub(super) fn input_mode(&self) -> Mode {
        Mode {
            english: self.english,
            traditional: self.config.traditional,
            full_width: self.full_width_for(self.english),
            scheme: self.scheme_name(),
        }
    }

    pub(super) fn hide_candidate_window(&mut self) {
        self.last_rect = None;
        self.last_shown = None;
        self.candidates.hide();
        self.hide_panel_window();
    }

    pub(super) fn position_candidates(&mut self, session: SessionId, rect: ScreenRect) {
        if self.focused != Some(session) {
            return;
        }
        self.last_rect = Some(rect);
        // 自绘窗吃未降级的帧（降级只作用于发给 DLL 的那份）
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
    }
}
