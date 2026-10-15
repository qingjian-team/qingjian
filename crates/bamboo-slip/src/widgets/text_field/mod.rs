//! 输入框（单行或多行）：文字、光标、选区与输入法的组字都画出来，编辑交给使用方保存的 [`TextEdit`]。
//!
//! 光标与选区不用量字宽：每行文字按光标、选区切成几段横排，光标是段与段之间的一条竖线，由布局摆位置。
//! 文字比框长时框里裁掉；光标在末尾时整行靠右，打字时总能看到最后几个字。点击只拿焦点、不挪光标。

mod edit;

use std::hash::Hash;
use std::sync::Arc;

use taffy::{AlignItems, FlexDirection, JustifyContent, LengthPercentageAuto, NodeId, Size, Style};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, centered, over, padding, row, size, solid};
use crate::animation::Easing;
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::{Icon, Visual};
use crate::text::{Decoration, TextStyle};
use crate::ui::{Interact, KeyCode, Ui};

pub use edit::{Edit, TextEdit};

/// 多行框上下的留白（点）。
const MULTILINE_PADDING: f32 = 6.0;

/// 密文显示成的圆点。
const BULLET: char = '•';

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

    disabled: bool,
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
            disabled: false,
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
        let lines = self.lines(ui, theme, focused)?;
        let content = ui.scene().node(
            Style {
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
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
                justify_content: Some(if !multiline && overflowing && at_end {
                    JustifyContent::FLEX_END
                } else {
                    JustifyContent::FLEX_START
                }),
                align_items: Some(if multiline {
                    AlignItems::FLEX_START
                } else {
                    AlignItems::CENTER
                }),
                ..Style::default()
            },
            Visual::Group,
            &[content],
        )?;
        ui.scene().set_clip(view, 0.0);
        ui.interact(view, key.child("view"), Interact::new());
        let line_height = self.style(theme, colors.text).line_height;
        let (height, min_height, vertical) = if multiline {
            let rows = self.rows.max(1) as f32;
            let min = rows * line_height + theme.px(MULTILINE_PADDING * 2.0);
            (None, min, MULTILINE_PADDING)
        } else if self.filled {
            (Some(theme.px(theme.control_height + 4.0)), 0.0, 0.0)
        } else {
            (Some(theme.px(theme.control_height)), 0.0, 0.0)
        };
        let paint = if self.filled {
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
        let node = ui.scene().node(
            Style {
                padding: padding(theme, vertical, if self.filled { 9.0 } else { 8.0 }),
                size: size(self.width.map(|width| theme.px(width)), height),
                min_size: Size {
                    width: LengthPercentageAuto::auto(),
                    height: LengthPercentageAuto::length(min_height),
                },
                flex_shrink: 0.0,
                align_items: Some(if multiline {
                    AlignItems::STRETCH
                } else {
                    AlignItems::CENTER
                }),
                ..row(theme, 7.0)
            },
            paint,
            &children,
        )?;
        ui.fade(node, key, theme.hover, Easing::Ease);
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        let secret = self.secret;
        let editing = on_edit.clone();
        let commit = on_commit.clone();
        let mut interact = Interact::new()
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

    /// 每行一排：选区前、选区、选区后，或光标前、组字、光标、光标后。单行就是一排。
    fn lines<M>(
        &self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        focused: bool,
    ) -> Result<Vec<NodeId>, RenderError> {
        let colors = &theme.colors;
        let text = self.edit.text();
        let preedit = self.edit.preedit();
        if text.is_empty() && preedit.is_empty() {
            let mut pieces = Vec::new();
            if focused {
                pieces.push(self.caret(ui, theme)?);
            }
            if !self.placeholder.is_empty() {
                pieces.push(self.segment(ui, theme, self.placeholder, colors.secondary, None)?);
            }
            return Ok(vec![self.line(ui, theme, &pieces)?]);
        }
        let selection = self.edit.selection();
        let caret = self.edit.caret();
        let mut lines = Vec::new();
        let mut start = 0;
        for line in text.split('\n') {
            let end = start + line.len();
            let mut pieces = Vec::new();
            if !focused {
                pieces.push(self.segment(ui, theme, line, colors.text, None)?);
            } else if !selection.is_empty() {
                let (from, to) = (
                    selection.start.clamp(start, end),
                    selection.end.clamp(start, end),
                );
                pieces.push(self.segment(ui, theme, &text[start..from], colors.text, None)?);
                if from < to {
                    let selected = self.segment(ui, theme, &text[from..to], colors.text, None)?;
                    pieces.push(ui.scene().node(
                        Style::default(),
                        solid(Some(colors.selected), None, theme.px(2.0)),
                        &[selected],
                    )?);
                }
                pieces.push(self.segment(ui, theme, &text[to..end], colors.text, None)?);
            } else if (start..=end).contains(&caret) {
                pieces.push(self.segment(ui, theme, &text[start..caret], colors.text, None)?);
                pieces.push(self.segment(
                    ui,
                    theme,
                    preedit,
                    colors.text,
                    Some(Decoration::Underline),
                )?);
                pieces.push(self.caret(ui, theme)?);
                pieces.push(self.segment(ui, theme, &text[caret..end], colors.text, None)?);
            } else {
                pieces.push(self.segment(ui, theme, line, colors.text, None)?);
            }
            lines.push(self.line(ui, theme, &pieces)?);
            start = end + 1;
        }
        Ok(lines)
    }

    /// 一行：各段横排，空行也有一行高。
    fn line<M>(
        &self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        pieces: &[NodeId],
    ) -> Result<NodeId, RenderError> {
        let height = self.style(theme, theme.colors.text).line_height;
        ui.scene().node(
            Style {
                flex_shrink: 0.0,
                align_items: Some(AlignItems::CENTER),
                min_size: Size {
                    width: LengthPercentageAuto::auto(),
                    height: LengthPercentageAuto::length(height),
                },
                ..row(theme, 0.0)
            },
            Visual::Group,
            pieces,
        )
    }

    fn caret<M>(&self, ui: &mut Ui<M>, theme: &WidgetTheme) -> Result<NodeId, RenderError> {
        // 熄灭的半拍只换透明色，不拿掉节点：文字不跟着挪
        let color = if ui.caret_visible() {
            theme.colors.accent
        } else {
            theme.colors.accent.transparent()
        };
        ui.scene().node(
            Style {
                size: size(Some(theme.px(1.5)), Some(theme.px(16.0))),
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::solid(color, 0.0),
            &[],
        )
    }

    /// 一段文字（密文换成圆点）；空串也建（占位为零宽），各段的位置才稳定。
    fn segment<M>(
        &self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        text: &str,
        color: Color,
        decoration: Option<Decoration>,
    ) -> Result<NodeId, RenderError> {
        let mut style = self.style(theme, color);
        if let Some(decoration) = decoration {
            style = style.decorated(decoration);
        }
        let shown = if self.secret && decoration.is_none() && color == theme.colors.text {
            std::iter::repeat_n(BULLET, text.chars().count()).collect()
        } else {
            text.to_owned()
        };
        ui.scene().node(
            Style {
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::Text { text: shown, style },
            &[],
        )
    }
}
