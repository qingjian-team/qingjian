//! 输入框（单行或多行）：文字、光标、选区与输入法的组字都画出来，编辑交给使用方保存的 [`TextEdit`]。
//!
//! 光标与选区不用量字宽：文字按 [`tokens`] 切成一个个单位横排，光标是单位之间的一条竖线，由布局摆位置；
//! 点击也靠这些单位：点中哪个单位、点在它的哪一侧，光标就落到哪（行尾后面的空白落到行尾，最后一行下面落到末尾）。
//! 单行：文字比框长时框里裁掉；光标在末尾时整行靠右，打字时总能看到最后几个字。
//! 多行：按框宽折行；设了最多几行时超出的裁掉，内容上移到能看见光标那一行。

mod edit;
mod flow;

use std::hash::Hash;
use std::ops::Range;
use std::sync::Arc;

use taffy::{
    AlignItems, FlexDirection, JustifyContent, LengthPercentageAuto, NodeId, Position, Rect, Size,
    Style,
};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, centered, over, padding, row, size, solid};
use crate::animation::Easing;
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::{Icon, Visual};
use crate::text::TextStyle;
use crate::ui::{Interact, Key, KeyCode, Ui};

pub use edit::{Edit, TextEdit};
pub use flow::{Flow, FlowTarget, TextFlow};

/// 多行框上下的留白（点）。
const MULTILINE_PADDING: f32 = 6.0;

/// 多行框里一段连着的 ASCII 最多几个字符一个折行单位：网址这类没有空格的长串也能折开。
const MAX_WORD: usize = 16;

pub struct TextField<'a> {
    edit: &'a TextEdit,

    placeholder: &'a str,

    /// 密文：显示成圆点，不能复制、剪切。
    secret: bool,

    /// 宽度（点）；`None` 撑满父节点给的宽度。
    width: Option<f32>,

    /// 文字前面的图标（搜索框的放大镜）。
    icon: Option<Arc<dyn Icon>>,

    /// 浅底无边框的样子（搜索框），拿到焦点时才描边。
    filled: bool,

    /// 多行时至少显示几行高（字多了往下长）。
    rows: usize,

    /// 多行时最多显示几行高；`None` 不设上限。
    max_rows: Option<usize>,

    disabled: bool,

    autofocus: bool,
}

impl<'a> TextField<'a> {
    pub fn new(edit: &'a TextEdit) -> Self {
        Self {
            edit,
            placeholder: "",
            secret: false,
            width: Some(240.0),
            icon: None,
            filled: false,
            rows: 3,
            max_rows: None,
            disabled: false,
            autofocus: false,
        }
    }

    /// 见 [`Interact::autofocus`]。
    pub fn autofocus(self) -> Self {
        Self {
            autofocus: true,
            ..self
        }
    }

    /// 空着时显示的灰字。
    pub fn placeholder(self, placeholder: &'a str) -> Self {
        Self {
            placeholder,
            ..self
        }
    }

    pub fn secret(self, secret: bool) -> Self {
        Self { secret, ..self }
    }

    pub fn width(self, points: f32) -> Self {
        Self {
            width: Some(points),
            ..self
        }
    }

    /// 撑满父节点给的宽度。
    pub fn stretch(self) -> Self {
        Self {
            width: None,
            ..self
        }
    }

    pub fn icon(self, icon: Arc<dyn Icon>) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    /// 浅底无边框（搜索框），拿到焦点时才描边。
    pub fn filled(self) -> Self {
        Self {
            filled: true,
            ..self
        }
    }

    /// 多行（编辑状态要是 [`TextEdit::multiline`]）至少几行高。
    pub fn rows(self, rows: usize) -> Self {
        Self { rows, ..self }
    }

    /// 多行最多几行高：再多时框里只显示光标附近的几行，免得长文字把框撑得比窗口还高。
    pub fn max_rows(self, rows: usize) -> Self {
        Self {
            max_rows: Some(rows),
            ..self
        }
    }

    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// `on_edit` 收每次编辑；回车（单行时）与失去焦点产出 `on_commit`（使用方此时保存，不需要时给 `None`）；
    /// 有 `on_cancel` 时 Esc 产出它（使用方把编辑状态换回保存的值），没有时 Esc 照常往上冒。
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_edit: impl Fn(Edit) -> M + Clone + 'static,
        on_commit: Option<M>,
        on_cancel: Option<M>,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let focused = state.focused && !self.disabled;
        let multiline = self.edit.is_multiline();
        let lines = self.lines(ui, theme, focused, key, &on_edit)?;
        // 多行限了高：光标那一行跑到框下面时，按上一帧量出的位置把内容往上推到正好露出它
        let lifted = match (
            ui.rect(key.child("caret")),
            ui.rect(key.child("line")),
            ui.rect(key.child("view")),
        ) {
            (Some(caret), Some(content), Some(view))
                if multiline && focused && self.max_rows.is_some() =>
            {
                let line_height = self.style(theme, colors.text).line_height;
                let bottom = caret.1 - content.1 + (caret.3 + line_height) / 2.0;
                (bottom - view.3).max(0.0)
            }
            _ => 0.0,
        };
        let content = ui.scene().node(
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
            &lines,
        )?;
        // 单行：光标在末尾、字又比框长时靠右；上一帧量出来的宽度够用，差一帧只在刚好超出的那个字上
        let overflowing = match (ui.rect(key.child("line")), ui.rect(key.child("view"))) {
            (Some(line), Some(view)) => line.2 > view.2,
            _ => false,
        };
        let at_end = self.edit.caret() == self.edit.text().len();
        ui.interact(content, key.child("line"), Interact::new());
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
        let mut interact = Interact::new();
        if !self.disabled {
            let on_edit = on_edit.clone();
            let end = self.edit.text().len();
            interact = interact.on_drag(move |_| Some(on_edit(Edit::Caret(end))));
        }
        ui.interact(view, key.child("view"), interact);
        let line_height = self.style(theme, colors.text).line_height;
        let tall =
            |rows: usize| rows.max(1) as f32 * line_height + theme.px(MULTILINE_PADDING * 2.0);
        let (height, min_height, vertical) = if multiline {
            (None, tall(self.rows), MULTILINE_PADDING)
        } else if self.filled {
            let extra = if theme.input_underline { 0.0 } else { 4.0 };
            (Some(theme.px(theme.control_height + extra)), 0.0, 0.0)
        } else {
            (Some(theme.px(theme.control_height)), 0.0, 0.0)
        };
        let max_height = match self.max_rows {
            Some(rows) if multiline => LengthPercentageAuto::length(tall(rows.max(self.rows))),
            _ => LengthPercentageAuto::auto(),
        };
        let paint = if theme.input_underline {
            let fill = if focused {
                colors.surface
            } else if state.hovered {
                over(colors.well, colors.hover)
            } else {
                colors.well
            };
            solid(
                Some(fill),
                Some((theme.px(1.0), colors.line)),
                theme.px(theme.radius),
            )
        } else if self.filled {
            let fill = if state.hovered && !focused {
                over(colors.well, colors.hover)
            } else {
                colors.well
            };
            let border = focused.then(|| (theme.px(1.5), colors.accent));
            solid(Some(fill), border, theme.px(theme.radius + 1.0))
        } else {
            let border = if focused { colors.accent } else { colors.line };
            solid(
                Some(colors.surface),
                Some((theme.px(if focused { 1.5 } else { 1.0 }), border)),
                theme.px(theme.radius),
            )
        };
        let mut children = Vec::new();
        if let Some(icon) = self.icon.clone() {
            let side = theme.px(13.0);
            children.push(ui.scene().node(
                centered(side, side),
                Visual::Icon {
                    icon,
                    size: side,
                    color: colors.secondary,
                },
                &[],
            )?);
        }
        children.push(view);
        if theme.input_underline && focused {
            children.push(ui.scene().node(
                Style {
                    position: Position::Absolute,
                    inset: Rect {
                        left: LengthPercentageAuto::length(0.0),
                        right: LengthPercentageAuto::length(0.0),
                        bottom: LengthPercentageAuto::length(0.0),
                        top: LengthPercentageAuto::auto(),
                    },
                    size: size(None, Some(theme.px(2.0))),
                    ..Style::default()
                },
                solid(Some(colors.accent), None, 0.0),
                &[],
            )?);
        }
        let node = ui.scene().node(
            Style {
                padding: padding(theme, vertical, if self.filled { 9.0 } else { 8.0 }),
                size: size(self.width.map(|width| theme.px(width)), height),
                min_size: Size {
                    width: LengthPercentageAuto::auto(),
                    height: LengthPercentageAuto::length(min_height),
                },
                max_size: Size {
                    width: LengthPercentageAuto::auto(),
                    height: max_height,
                },
                flex_shrink: 0.0,
                align_items: if multiline {
                    AlignItems::STRETCH
                } else {
                    AlignItems::CENTER
                },
                ..row(theme, 7.0)
            },
            paint,
            &children,
        )?;
        ui.fade(node, key, theme.hover, Easing::Ease);
        if theme.input_underline {
            ui.scene().set_clip(node, theme.px(theme.radius));
        }
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        let secret = self.secret;
        let editing = on_edit.clone();
        let commit = on_commit.clone();
        let mut interact = Interact::new()
            .secret(secret)
            .autofocus(self.autofocus)
            .on_text(move |input| Some(on_edit(Edit::Text(input.clone()))))
            .on_key(move |press| {
                let command = press.modifiers.command;
                match &press.key {
                    KeyCode::Enter if multiline => Some(editing(Edit::Key(press.clone()))),
                    KeyCode::Enter => commit.clone(),
                    KeyCode::Escape => on_cancel.clone(),
                    KeyCode::ArrowUp | KeyCode::ArrowDown if multiline => {
                        Some(editing(Edit::Key(press.clone())))
                    }
                    KeyCode::ArrowLeft
                    | KeyCode::ArrowRight
                    | KeyCode::Home
                    | KeyCode::End
                    | KeyCode::Backspace
                    | KeyCode::Delete => Some(editing(Edit::Key(press.clone()))),
                    KeyCode::Character(text)
                        if command
                            && (text == "a" || (!secret && (text == "c" || text == "x"))) =>
                    {
                        Some(editing(Edit::Key(press.clone())))
                    }
                    _ => None,
                }
            })
            .disabled(self.disabled);
        if let Some(commit) = on_commit {
            interact = interact.on_blur(commit);
        }
        ui.interact(node, key, interact);
        Ok(node)
    }

    fn style(&self, theme: &WidgetTheme, color: Color) -> TextStyle {
        theme.text(theme.font.size - 1.0, 400, color)
    }

    /// 正文各行，排法在 [`TextFlow`]；光标登记成 `key` 下的 `caret`（限高时按它挪内容），单位与行尾接点击。
    fn lines<M: Clone + 'static>(
        &self,
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
            text: self.edit.text(),
            preedit: self.edit.preedit(),
            caret: self.edit.caret(),
            selection: self.edit.selection(),
            placeholder: self.placeholder,
            multiline: self.edit.is_multiline(),
            focused,
            secret: self.secret,
            style: self.style(theme, colors.text),
            placeholder_style: self.style(theme, colors.secondary),
            caret_bar: (theme.px(1.5), theme.px(16.0), caret_color),
            selection_paint: solid(Some(colors.selected), None, theme.px(2.0)),
        }
        .build(ui.scene())?;
        for target in flow.targets {
            match target {
                FlowTarget::Caret(node) => {
                    ui.interact(node, key.child("caret"), Interact::new());
                }
                _ if self.disabled => {}
                FlowTarget::Token { node, bounds } => {
                    let on_edit = on_edit.clone();
                    ui.interact(
                        node,
                        key.child(("at", bounds[0])),
                        Interact::new().on_drag(move |fraction| {
                            Some(on_edit(Edit::Caret(FlowTarget::caret_at(
                                &bounds, fraction,
                            ))))
                        }),
                    );
                }
                // 点在这一行的空白处（行尾后面）：光标到行尾
                FlowTarget::LineEnd { node, start, end } => {
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
}

/// 排版与点击的单位（字节范围）：一个非 ASCII 字符，或一段连着的 ASCII（最多 [`MAX_WORD`] 个字符）；
/// 后面跟着的空白归它，空白不会折到下一排的开头。组合用的字符（附加符号、变体选择符、肤色、零宽连接符
/// 和它后面那个字、国旗的第二个区域符号）跟着前一个字，不然 emoji 组合与带附加符号的字会被拆开画。
fn tokens(text: &str) -> Vec<Range<usize>> {
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
            && (spaced || wide || !c.is_ascii() || word >= MAX_WORD)
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
