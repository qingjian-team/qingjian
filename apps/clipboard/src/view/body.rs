//! 卡片中间的内容：文字保留换行与缩进并折行，代码等宽带高亮，图片按比例缩放，文件列出文件名。

use std::ops::Range;
use std::sync::Arc;

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentageAuto, NodeId, Size, Style,
};
use bamboo_slip::tiny_skia::Pixmap;
use bamboo_slip::{BoxPaint, Color, Fill, RenderError, Ui, Visual};
use qingjian_clipboard::{ClipItem, ClipKind};
use qingjian_code::{CodeKind, CodeSpan, highlight};

use super::{Context, pad, text_node};
use crate::app::Message;

/// 卡片正文最多排多少行、每行多少字符：放不下的本来也看不见，不用把整段长文排一遍。
const PREVIEW_LINES: usize = 16;

const PREVIEW_LINE_CHARS: usize = 400;

/// 代码卡片只高亮开头这么多字节：够排满预览，一兆的代码也不用每次重建都扫一遍。
const PREVIEW_BYTES: usize = PREVIEW_LINES * PREVIEW_LINE_CHARS * 4;

/// 一个制表符显示成几个空格。
const TAB: &str = "    ";

fn full_width() -> Size<Dimension> {
    Size {
        width: Dimension::percent(1.0),
        height: Dimension::auto(),
    }
}

fn text_lines(
    ui: &mut Ui<Message>,
    context: &Context,
    item: &ClipItem,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &context.theme;
    let style = theme
        .text(theme.font.size, 400, theme.colors.text)
        .wrapped();
    let mut lines = Vec::new();
    for line in item.text.lines().take(PREVIEW_LINES) {
        let line: String = line
            .replace('\t', TAB)
            .chars()
            .take(PREVIEW_LINE_CHARS)
            .collect();
        lines.push(text_node(
            ui,
            // 空行也要占一行高
            if line.is_empty() {
                " ".to_owned()
            } else {
                line
            },
            style,
            Style {
                flex_shrink: 0.0,
                size: full_width(),
                ..Style::default()
            },
        )?);
    }
    Ok(lines)
}

/// 把 `line` 这段按高亮片段切开；跨行的片段（块注释）只取落在这一行的部分。
fn line_segments(line: Range<usize>, spans: &[CodeSpan]) -> Vec<(Range<usize>, Option<CodeKind>)> {
    let mut segments = Vec::new();
    let mut at = line.start;
    for span in spans {
        let start = span.range.start.max(line.start);
        let end = span.range.end.min(line.end);
        if start >= end {
            continue;
        }
        if at < start {
            segments.push((at..start, None));
        }
        segments.push((start..end, Some(span.kind)));
        at = end;
    }
    if at < line.end {
        segments.push((at..line.end, None));
    }
    segments
}

fn code_lines(
    ui: &mut Ui<Message>,
    context: &Context,
    item: &ClipItem,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &context.theme;
    let colors = &context.palette.code;
    let mut cut = item.text.len().min(PREVIEW_BYTES);
    while !item.text.is_char_boundary(cut) {
        cut -= 1;
    }
    let preview = &item.text[..cut];
    let spans = highlight(preview);
    let mut base = theme
        .text(theme.font.size, 400, theme.colors.text)
        .wrapped();
    base.family = context.app.mono;
    let mut lines = Vec::new();
    let mut start = 0;
    for raw in preview.split_inclusive('\n').take(PREVIEW_LINES) {
        let end = start + raw.trim_end_matches(['\r', '\n']).len();
        let mut shown = String::new();
        let mut pieces = Vec::new();
        for (range, kind) in line_segments(start..end, &spans) {
            let color = match kind {
                Some(CodeKind::Keyword) => colors.keyword,
                Some(CodeKind::String) => colors.string,
                Some(CodeKind::Comment) => colors.comment,
                Some(CodeKind::Number) => colors.number,
                None => base.color,
            };
            let room = PREVIEW_LINE_CHARS.saturating_sub(shown.chars().count());
            let at = shown.len();
            shown.extend(preview[range].replace('\t', TAB).chars().take(room));
            if shown.len() > at {
                pieces.push((at..shown.len(), color));
            }
        }
        // 空行也要占一行高
        let visual = if pieces.is_empty() {
            Visual::Text {
                text: " ".to_owned(),
                style: base,
            }
        } else {
            Visual::RichText {
                text: shown,
                style: base,
                spans: pieces,
            }
        };
        lines.push(ui.scene().node(
            Style {
                flex_shrink: 0.0,
                size: full_width(),
                ..Style::default()
            },
            visual,
            &[],
        )?);
        start += raw.len();
    }
    Ok(lines)
}

fn file_lines(
    ui: &mut Ui<Message>,
    context: &Context,
    item: &ClipItem,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &context.theme;
    let mut lines = Vec::new();
    for path in item.text.lines().take(PREVIEW_LINES) {
        let name = std::path::Path::new(path).file_name().map_or_else(
            || path.to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let mut name_style = theme.text(theme.font.size, 500, theme.colors.text);
        name_style.truncate = true;
        lines.push(text_node(
            ui,
            name,
            name_style,
            Style {
                flex_shrink: 0.0,
                size: full_width(),
                ..Style::default()
            },
        )?);
    }
    Ok(lines)
}

/// 图片按比例缩到放得下，不放大。
fn fit(image: (f32, f32), room: (f32, f32), scale: f32) -> (f32, f32) {
    let ratio = (room.0 / image.0).min(room.1 / image.1).min(scale);
    (image.0 * ratio, image.1 * ratio)
}

fn image(
    ui: &mut Ui<Message>,
    context: &Context,
    pixmap: &Arc<Pixmap>,
    room: (f32, f32),
) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let (width, height) = fit(
        (pixmap.width() as f32, pixmap.height() as f32),
        room,
        theme.scale,
    );
    ui.scene().node(
        Style {
            flex_shrink: 0.0,
            size: Size {
                width: Dimension::length(width),
                height: Dimension::length(height),
            },
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Image {
                pixmap: pixmap.clone(),
                slice: None,
                pixels_per_px: 1.0,
            }),
            border: Some((theme.px(1.0), Color::gray(128, 40))),
            radius: 0.0,
        }),
        &[],
    )
}

pub(super) fn body(
    ui: &mut Ui<Message>,
    context: &Context,
    item: &ClipItem,
    image_room: (f32, f32),
) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let is_image = item.kind == ClipKind::Image;
    let children = match item.kind {
        ClipKind::Image => match context.app.thumbnails.get(&item.id) {
            Some(pixmap) => vec![image(ui, context, pixmap, image_room)?],
            None => Vec::new(),
        },
        ClipKind::Code => code_lines(ui, context, item)?,
        ClipKind::Files => file_lines(ui, context, item)?,
        ClipKind::Text | ClipKind::Rich => text_lines(ui, context, item)?,
    };
    let node = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_basis: Dimension::length(0.0),
            min_size: Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::length(0.0),
            },
            align_items: if is_image {
                AlignItems::CENTER
            } else {
                AlignItems::START
            },
            justify_content: if is_image {
                JustifyContent::CENTER
            } else {
                JustifyContent::START
            },
            padding: pad(theme, 8.0, 10.0),
            ..Style::default()
        },
        Visual::Group,
        &children,
    )?;
    ui.scene().set_clip(node, 0.0);
    Ok(node)
}

#[cfg(test)]
mod tests {
    use qingjian_code::{CodeKind, CodeSpan};

    use super::{fit, line_segments};

    #[test]
    fn segments_cover_the_line_and_clip_multiline_spans() {
        let spans = [
            CodeSpan {
                range: 2..4,
                kind: CodeKind::Keyword,
            },
            CodeSpan {
                range: 8..20,
                kind: CodeKind::Comment,
            },
        ];
        assert_eq!(
            line_segments(0..10, &spans),
            [
                (0..2, None),
                (2..4, Some(CodeKind::Keyword)),
                (4..8, None),
                (8..10, Some(CodeKind::Comment)),
            ]
        );
        assert_eq!(
            line_segments(10..14, &spans),
            [(10..14, Some(CodeKind::Comment))]
        );
        assert_eq!(line_segments(30..30, &spans), []);
    }

    #[test]
    fn images_shrink_to_fit_but_do_not_grow_past_their_size() {
        assert_eq!(fit((1000.0, 500.0), (500.0, 500.0), 1.0), (500.0, 250.0));
        assert_eq!(fit((100.0, 50.0), (500.0, 500.0), 1.0), (100.0, 50.0));
    }
}
