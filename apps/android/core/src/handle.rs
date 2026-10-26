//! JNI 句柄背后的东西：输入会话加候选条画笔。

use crate::frame::Frame;
use crate::session::Session;
use crate::strip::StripPainter;

pub struct Handle {
    pub session: Session,

    /// 字体加载失败时为 `None`，壳收到空的候选条说明就退回自己画。
    pub strip: Option<StripPainter>,
}

impl Handle {
    pub fn new(session: Session) -> Self {
        let strip = StripPainter::new()
            .inspect_err(|error| tracing::error!(%error, "候选条画笔建不起来"))
            .ok();
        Self { session, strip }
    }

    /// 取回云联想的结果插进候选，有新的一帧就让画笔记下并返回；没有返回 `None`。
    pub fn poll_prediction(&mut self) -> Option<Frame> {
        let frame = self.session.poll_prediction()?;
        if let Some(strip) = &mut self.strip {
            strip.show(&frame);
        }
        Some(frame)
    }

    /// 跑一个会话操作，顺手让画笔记下这一帧；之前先收释义兜底的结果，这一帧的译词就用得上。
    pub fn operate(&mut self, operate: impl FnOnce(&mut Session) -> Frame) -> Frame {
        self.session.poll_glosses();
        let frame = operate(&mut self.session);
        if let Some(strip) = &mut self.strip {
            strip.show(&frame);
        }
        frame
    }
}
