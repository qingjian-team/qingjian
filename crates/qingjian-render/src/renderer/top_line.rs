//! 顶部拼音行：各段按样式画、自己画光标、右侧整句补全或临时状态。

use super::{CARET_WIDTH, Metrics, Renderer, SENTENCE_GAP};
use crate::canvas::Canvas;
use crate::frame::{Frame, Preedit, PreeditStyle};

impl Renderer {
    /// 顶部拼音行（含右侧整句补全）需要的宽高；没有这一行时都是 0。
    pub(super) fn top_line_size(&mut self, frame: &Frame, m: &Metrics) -> (f32, f32) {
        if !frame.has_top_line() {
            return (0.0, 0.0);
        }
        let style = m.annotation_style(m.theme.colors.gloss);
        let line_height = style.line_height;
        let mut width = 0.0;
        if let Some(preedit) = &frame.preedit {
            width += self.measure(&preedit.text(), &style).width + m.px(CARET_WIDTH);
        }
        if let Some((text, cloud)) = frame.trailing() {
            if frame.preedit.is_some() {
                width += m.px(SENTENCE_GAP);
            }
            if cloud {
                width += m.cloud_width();
            }
            width += self.measure(text, &style).width;
        }
        (width, line_height + m.row_padding() * 2.0)
    }

    /// 返回占用高度。`left` 是内容区左边。
    pub(super) fn draw_top_line(
        &mut self,
        canvas: &mut Canvas,
        frame: &Frame,
        m: &Metrics,
        left: f32,
        y: f32,
    ) -> f32 {
        if !frame.has_top_line() {
            return 0.0;
        }
        let line_height = m.px(m.theme.annotation_font.line_height);
        let top = y + m.row_padding();
        let mut x = left + m.padding();
        if let Some(preedit) = &frame.preedit {
            x += self.draw_preedit(canvas, m, preedit, x, top, line_height);
            if frame.trailing().is_some() {
                x += m.px(SENTENCE_GAP);
            }
        }
        // 整句补全：云朵 + 句子，颜色与本地候选区分；临时状态灰字、不带云朵
        if let Some((text, cloud)) = frame.trailing() {
            let color = if cloud {
                x += self.draw_cloud(canvas, m, x, top, line_height);
                m.theme.colors.cloud
            } else {
                m.theme.colors.gloss
            };
            let style = m.annotation_style(color);
            self.draw_text(canvas, text, &style, x, top);
        }
        line_height + m.row_padding() * 2.0
    }

    /// 画拼音行的各段与光标，返回占用宽度（含光标）。
    fn draw_preedit(
        &mut self,
        canvas: &mut Canvas,
        m: &Metrics,
        preedit: &Preedit,
        x: f32,
        top: f32,
        line_height: f32,
    ) -> f32 {
        let mut cursor_x = x;
        for segment in &preedit.segments {
            let style = match segment.style {
                PreeditStyle::Typed => m.annotation_style(m.theme.colors.gloss),
                PreeditStyle::Rest => m.annotation_style(m.theme.colors.pos),
                PreeditStyle::Struck => m.annotation_style(m.theme.colors.pos).struck(),
                PreeditStyle::AuxCode => m.annotation_style(m.theme.colors.pos).underlined(),
            };
            cursor_x += self.draw_text(canvas, &segment.text, &style, cursor_x, top);
        }
        let measure_style = m.annotation_style(m.theme.colors.gloss);
        let caret_x = x + self.measure(&preedit.before_cursor(), &measure_style).width;
        canvas.fill_rect(
            caret_x,
            top,
            m.px(CARET_WIDTH),
            line_height,
            m.theme.colors.text,
        );
        cursor_x - x + m.px(CARET_WIDTH)
    }
}
