//! 竖排：一行一个候选，序号 / 候选词 / 译文三列；上方窗口可按行倒序，页码放在顶部。

use super::columns::Columns;
use super::{Metrics, Renderer};
use crate::canvas::Canvas;
use crate::frame::{Frame, Row};

impl Renderer {
    pub(super) fn vertical_size(&mut self, frame: &Frame, m: &Metrics) -> (f32, f32) {
        let columns = self.columns(&frame.rows, m);
        let mut width = columns.index_width + m.column_gap() + columns.text_width;
        if columns.annotation_width > 0.0 {
            width += m.column_gap() + columns.annotation_width;
        }
        let mut height = columns.row_height * frame.rows.len() as f32;
        if let Some(footer) = frame.footer.as_deref() {
            let footer_size = self.measure(footer, &m.index_style());
            width = width.max(footer_size.width);
            height += footer_size.height + m.row_padding();
        }
        (width, height)
    }

    fn columns(&mut self, rows: &[Row], m: &Metrics) -> Columns {
        let mut columns = Columns {
            index_width: 0.0,
            text_width: 0.0,
            annotation_width: 0.0,
            row_height: 0.0,
        };
        let text_style = m.text_style();
        let index_style = m.index_style();
        let annotation_style = m.annotation_style(m.theme.colors.gloss);
        for row in rows {
            let index = self.measure(&row.index, &index_style);
            let mut text = self.measure(&row.text, &text_style);
            if row.cloud {
                text.width += m.cloud_width();
            }
            text.width += self.code_width(row, m);
            let annotation: f32 = row
                .annotation
                .iter()
                .map(|(s, _)| self.measure(s, &annotation_style).width)
                .sum();
            columns.index_width = columns.index_width.max(index.width);
            columns.text_width = columns.text_width.max(text.width);
            columns.annotation_width = columns.annotation_width.max(annotation);
            columns.row_height = columns.row_height.max(text.height + m.row_padding() * 2.0);
        }
        columns
    }

    pub(super) fn draw_vertical(
        &mut self,
        canvas: &mut Canvas,
        frame: &Frame,
        m: &Metrics,
        origin: (f32, f32),
        content_width: f32,
        reversed: bool,
    ) -> f32 {
        let (left, y) = origin;
        // 量尺寸时已整形过一遍，这里再整形一遍；等渲染器定型再把结果从 render 传下来。
        let columns = self.columns(&frame.rows, m);
        let text_x = left + m.padding() + columns.index_width + m.column_gap();
        let annotation_x = text_x + columns.text_width + m.column_gap();
        let text_height = m.px(m.theme.text_font.line_height);
        let footer_height = frame.footer.as_deref().map_or(0.0, |footer| {
            self.measure(footer, &m.index_style()).height + m.row_padding()
        });
        let rows_height = columns.row_height * frame.rows.len() as f32;
        let rows_y = y + if reversed { footer_height } else { 0.0 };
        for (i, row) in frame.rows.iter().enumerate() {
            let position = if reversed {
                frame.rows.len() - 1 - i
            } else {
                i
            };
            let y = rows_y + position as f32 * columns.row_height;
            if Some(i) == frame.highlighted {
                self.fill_highlight(
                    canvas,
                    m,
                    left + m.padding() / 2.0,
                    y,
                    content_width - m.padding(),
                    columns.row_height,
                );
            }
            let top = y + m.row_padding();
            let small_offset = m.small_offset(text_height);
            self.draw_text(
                canvas,
                &row.index,
                &m.index_style(),
                left + m.padding(),
                top + small_offset,
            );
            self.draw_word(canvas, m, row, text_x, top, text_height);
            let mut x = annotation_x;
            for (segment, tone) in &row.annotation {
                let style = m.annotation_style(m.tone_color(*tone));
                x += self.draw_text(canvas, segment, &style, x, top + small_offset);
            }
        }
        if let Some(footer) = frame.footer.as_deref() {
            let style = m.index_style();
            let size = self.measure(footer, &style);
            self.draw_text(
                canvas,
                footer,
                &style,
                left + content_width - m.padding() - size.width,
                y + if reversed { 0.0 } else { rows_height } + m.row_padding(),
            );
        }
        rows_height + footer_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::FontLibrary;
    use crate::frame::{Preedit, Tone};
    use crate::layout::Layout;
    use crate::theme::Theme;
    use tiny_skia::Pixmap;

    fn band(pixmap: &Pixmap, y: u32, height: u32, padding: u32) -> Vec<u8> {
        let width = pixmap.width() as usize;
        (y..y + height)
            .flat_map(|row| {
                let start = (row as usize * width + padding as usize) * 4;
                let end = ((row as usize + 1) * width - padding as usize) * 4;
                pixmap.data()[start..end].iter().copied()
            })
            .collect()
    }

    #[test]
    fn reversed_layout_moves_whole_bands_without_mirroring_glyphs_or_highlight() {
        let library = FontLibrary::system("zh-CN").expect("渲染回归需要系统字体");
        let mut renderer = Renderer::new(library);
        let frame = Frame {
            preedit: Some(Preedit::plain("ce'shi", 6)),
            rows: vec![
                Row {
                    annotation: vec![("test".into(), Tone::Gloss)],
                    ..Row::plain(0, "测试")
                },
                Row::plain(1, ""),
                Row {
                    cloud: true,
                    ..Row::plain(2, "联想")
                },
            ],
            highlighted: Some(0),
            footer: Some("1/56".into()),
            ..Frame::default()
        };
        let original = frame.clone();
        for theme in [Theme::light(), Theme::dark()] {
            for scale in [1.0, 2.0] {
                let m = Metrics {
                    theme: &theme,
                    scale,
                };
                let (_, top) = renderer.top_line_size(&frame, &m);
                let columns = renderer.columns(&frame.rows, &m);
                let row_height = columns.row_height as u32;
                let footer =
                    (renderer.measure("1/56", &m.index_style()).height + m.row_padding()) as u32;
                let padding = m.padding() as u32;
                let top = top as u32;
                let rows = row_height * frame.rows.len() as u32;
                let normal = renderer
                    .render(&frame, Layout::Vertical, &theme, scale, None)
                    .unwrap();
                let reversed = renderer
                    .render_vertical_reversed(&frame, &theme, scale, None)
                    .unwrap();
                assert_eq!(normal.content_size_points(), reversed.content_size_points());
                assert_eq!(
                    band(&normal.pixmap, padding, top, padding),
                    band(&reversed.pixmap, padding + footer + rows, top, padding)
                );
                assert_eq!(
                    band(&normal.pixmap, padding + top + rows, footer, padding),
                    band(&reversed.pixmap, padding, footer, padding)
                );
                for i in 0..frame.rows.len() as u32 {
                    assert_eq!(
                        band(
                            &normal.pixmap,
                            padding + top + i * row_height,
                            row_height,
                            padding
                        ),
                        band(
                            &reversed.pixmap,
                            padding + footer + (frame.rows.len() as u32 - 1 - i) * row_height,
                            row_height,
                            padding
                        ),
                    );
                }
            }
        }
        assert_eq!(frame, original);
    }

    #[test]
    fn missing_sections_and_no_highlight_keep_the_same_window_size() {
        let library = FontLibrary::system("zh-CN").expect("渲染回归需要系统字体");
        let mut renderer = Renderer::new(library);
        for preedit in [None, Some(Preedit::plain("a", 1))] {
            for footer in [None, Some("1/2".to_owned())] {
                for rows in [vec![], vec![Row::plain(0, "啊")]] {
                    let frame = Frame {
                        preedit: preedit.clone(),
                        footer: footer.clone(),
                        rows,
                        ..Frame::default()
                    };
                    let normal = renderer
                        .render(&frame, Layout::Vertical, &Theme::light(), 2.0, None)
                        .unwrap();
                    let reversed = renderer
                        .render_vertical_reversed(&frame, &Theme::light(), 2.0, None)
                        .unwrap();
                    assert_eq!(normal.content_size_points(), reversed.content_size_points());
                    assert_eq!(normal.pixmap.width(), reversed.pixmap.width());
                    assert_eq!(normal.pixmap.height(), reversed.pixmap.height());
                }
            }
        }
    }
}
