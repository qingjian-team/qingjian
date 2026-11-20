//! 候选窗口一次绘制要用的内容：配色与字体（GDI 画法用）＋展示数据。
//!
//! 展示数据（拼音行、候选行、页码、整句补全）由 `qingjian_render::CandidateView` 从协议帧换算，
//! 与 TSF 进程内候选窗共用同一份换算，免得两处各写一遍「序号 / 译词 / 矩阵标号」。

use std::ops::Deref;
use std::rc::Rc;

use qingjian_platform::protocol::Frame;
use qingjian_render::{CandidateView, Mode};

use super::theme::Theme;

/// 一次绘制要用的全部内容。
pub(crate) struct RenderData {
    /// 配色与字体（随 DPI / 深浅重建）。
    pub(super) theme: Rc<Theme>,

    /// 帧换算出来的展示数据。
    view: CandidateView,
}

impl RenderData {
    pub(super) fn empty(theme: Rc<Theme>) -> Self {
        Self {
            theme,
            view: CandidateView::default(),
        }
    }

    pub(super) fn set(&mut self, frame: &Frame, mode: Mode) {
        self.view.set(frame, mode);
    }

    /// 渲染器要的帧。
    pub(super) fn render_frame(&self) -> qingjian_render::Frame {
        self.view.render_frame()
    }
}

/// 绘制代码按展示数据的字段读（`data.rows`、`data.preedit`……），与搬去共享层之前一样。
impl Deref for RenderData {
    type Target = CandidateView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}
