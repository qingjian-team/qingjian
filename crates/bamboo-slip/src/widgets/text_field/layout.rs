//! 输入框正文的布局：各行节点、限高时的上推量、内容层与视口层，以及排版单位的切分。

use std::ops::Range;

use taffy::{AlignItems, FlexDirection, JustifyContent, LengthPercentageAuto, NodeId, Rect, Style};

use super::super::parts::solid;
use super::{Edit, TextField, TextFlow, WidgetTheme};
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Key, Ui};

/// 正文各行，排法在 [`TextFlow`]；光标登记成 `key` 下的 `caret`（限高时按它挪内容），单位与行尾接点击。
pub(super) fn lines<M: Clone + 'static>(
    field: &TextField,
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    focused: bool,
    key: Key,
    on_edit: &(impl Fn(Edit) -> M + Clone + 'static),
) -> Result<Vec<NodeId>, RenderError> {
    let colors = &theme.colors;
    // 熄灭的半拍只换透明色，不拿掉节点：文字不跟着挪
    let caret_color = if ui.caret_visible() {
        colors.accent
    } else {
        colors.accent.transparent()
    };
    let flow = TextFlow {
        text: field.edit.text(),
        preedit: field.edit.preedit(),
        caret: field.edit.caret(),
        selection: field.edit.selection(),
        placeholder: field.placeholder,
        multiline: field.edit.is_multiline(),
        focused,
        secret: field.secret,
        style: field.style(theme, colors.text),
        placeholder_style: field.style(theme, colors.secondary),
        caret_bar: (theme.px(1.5), theme.px(16.0), caret_color),
        selection_paint: solid(Some(colors.selected), None, theme.px(2.0)),
    }
    .build(ui.scene())?;
    for target in flow.targets {
        match target {
            super::FlowTarget::Caret(node) => {
                ui.interact(node, key.child("caret"), Interact::new());
            }
            _ if field.disabled => {}
            super::FlowTarget::Token { node, bounds } => {
                let on_edit = on_edit.clone();
                ui.interact(
                    node,
                    key.child(("at", bounds[0])),
                    Interact::new().on_drag(move |fraction| {
                        Some(on_edit(Edit::Caret(super::FlowTarget::caret_at(
                            &bounds, fraction,
                        ))))
                    }),
                );
            }
            // 点在这一行的空白处（行尾后面）：光标到行尾
            super::FlowTarget::LineEnd { node, start, end } => {
                let on_edit = on_edit.clone();
                ui.interact(
                    node,
                    key.child(("line-end", start)),
                    Interact::new().on_drag(move |_| Some(on_edit(Edit::Caret(end)))),
                );
            }
        }
    }
    Ok(flow.lines)
}

/// 多行限了高：光标那一行跑到框下面时，按上一帧量出的位置把内容往上推到正好露出它。
pub(super) fn lifted<M>(
    ui: &Ui<M>,
    theme: &WidgetTheme,
    multiline: bool,
    focused: bool,
    capped: bool,
    key: Key,
) -> f32 {
    match (
        ui.rect(key.child("caret")),
        ui.rect(key.child("line")),
        ui.rect(key.child("view")),
    ) {
        (Some(caret), Some(content), Some(view)) if multiline && focused && capped => {
            let line_height = theme
                .text(theme.font.size - 1.0, 400, theme.colors.text)
                .line_height;
            let bottom = caret.1 - content.1 + (caret.3 + line_height) / 2.0;
            (bottom - view.3).max(0.0)
        }
        _ => 0.0,
    }
}

/// 内容层：把各行竖排，限高时整体上推 `lifted`。
pub(super) fn content<M>(
    ui: &mut Ui<M>,
    lifted: f32,
    lines: &[NodeId],
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            margin: Rect {
                top: LengthPercentageAuto::length(-lifted),
                ..Rect::zero()
            },
            ..Style::default()
        },
        Visual::Group,
        lines,
    )
}

/// 视口层：单行溢出且光标在末尾时整行靠右；多行内容拉满框宽才有宽度可折行。
pub(super) fn view<M>(
    ui: &mut Ui<M>,
    multiline: bool,
    content: NodeId,
    overflowing: bool,
    at_end: bool,
) -> Result<NodeId, RenderError> {
    let view = ui.scene().node(
        Style {
            flex_grow: 1.0,
            flex_direction: if multiline {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            },
            justify_content: if !multiline && overflowing && at_end {
                JustifyContent::FLEX_END
            } else {
                JustifyContent::FLEX_START
            },
            // 多行的内容拉满框宽，才有宽度可折行
            align_items: if multiline {
                AlignItems::STRETCH
            } else {
                AlignItems::CENTER
            },
            ..Style::default()
        },
        Visual::Group,
        &[content],
    )?;
    ui.scene().set_clip(view, 0.0);
    Ok(view)
}

/// 排版与点击的单位（字节范围）：一个非 ASCII 字符，或一段连着的 ASCII（最多 [`super::MAX_WORD`] 个字符）；
/// 后面跟着的空白归它，空白不会折到下一排的开头。组合用的字符（附加符号、变体选择符、肤色、零宽连接符
/// 和它后面那个字、国旗的第二个区域符号）跟着前一个字，不然 emoji 组合与带附加符号的字会被拆开画。
pub(super) fn tokens(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut word = 0;
    let mut spaced = false;
    let mut wide = false;
    let mut joined = false;
    let mut flag = false;
    for (i, c) in text.char_indices() {
        let attached = joined || extends(c) || (flag && is_regional(c));
        if i > start
            && !attached
            && !c.is_whitespace()
            && (spaced || wide || !c.is_ascii() || word >= super::MAX_WORD)
        {
            out.push(start..i);
            (start, word, spaced, wide) = (i, 0, false, false);
        }
        joined = c == '\u{200D}';
        // 区域符号两个一对：配成一面旗以后，下一个区域符号另起
        flag = is_regional(c) && !(flag && attached);
        if c.is_whitespace() {
            spaced = true;
        } else if c.is_ascii() {
            word += 1;
        } else {
            wide = true;
        }
    }
    if start < text.len() {
        out.push(start..text.len());
    }
    out
}

/// 跟在前一个字上、自己不单独成字的字符。
fn extends(c: char) -> bool {
    matches!(
        c,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{200D}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FE20}'..='\u{FE2F}'
            | '\u{1F3FB}'..='\u{1F3FF}'
            | '\u{E0020}'..='\u{E007F}'
            | '\u{E0100}'..='\u{E01EF}'
    )
}

/// 区域指示符：两个拼成一面国旗。
fn is_regional(c: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)
}

#[cfg(test)]
mod tests {
    use super::tokens;

    fn split(text: &str) -> Vec<&str> {
        tokens(text).into_iter().map(|range| &text[range]).collect()
    }

    #[test]
    fn wide_characters_and_words_are_wrap_units() {
        assert_eq!(
            split("你好 hello world，再见"),
            ["你", "好 ", "hello ", "world", "，", "再", "见"]
        );
        assert_eq!(
            split("https://example.com/a/b/c"),
            ["https://example.", "com/a/b/c"]
        );
        assert_eq!(split("  a"), ["  ", "a"]);
        assert!(tokens("").is_empty());
    }

    #[test]
    fn combining_sequences_stay_together() {
        // 一家三口（零宽连接）、竖大拇指加肤色、两面国旗、带声调的 a
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F466}";
        let thumb = "\u{1F44D}\u{1F3FD}";
        let (china, japan) = ("\u{1F1E8}\u{1F1F3}", "\u{1F1EF}\u{1F1F5}");
        let text = format!("好{family}{thumb}{china}{japan}a\u{0301}");
        assert_eq!(
            split(&text),
            ["好", family, thumb, china, japan, "a\u{0301}"]
        );
    }
}
