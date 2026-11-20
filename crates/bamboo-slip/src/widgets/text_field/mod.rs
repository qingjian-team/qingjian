//! 输入框（单行或多行）：文字、光标、选区与输入法的组字都画出来，编辑交给使用方保存的 [`TextEdit`]。
//!
//! 光标与选区不用量字宽：文字按 [`layout::tokens`] 切成一个个单位横排，光标是单位之间的一条竖线，由布局摆位置；
//! 点击也靠这些单位：点中哪个单位、点在它的哪一侧，光标就落到哪（行尾后面的空白落到行尾，最后一行下面落到末尾）。
//! 单行：文字比框长时框里裁掉；光标在末尾时整行靠右，打字时总能看到最后几个字。
//! 多行：按框宽折行；设了最多几行时超出的裁掉，内容上移到能看见光标那一行。

mod edit;
mod flow;
mod input;
mod layout;
mod paint;

use std::hash::Hash;
use std::sync::Arc;

use taffy::NodeId;

use super::WidgetTheme;
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::Icon;
use crate::text::TextStyle;
use crate::ui::Ui;

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
        let focused = state.focused && !self.disabled;
        let multiline = self.edit.is_multiline();
        let lines = layout::lines(&self, ui, theme, focused, key, &on_edit)?;
        let lifted = layout::lifted(ui, theme, multiline, focused, self.max_rows.is_some(), key);
        let content = layout::content(ui, lifted, &lines)?;
        // 单行：光标在末尾、字又比框长时靠右；上一帧量出来的宽度够用，差一帧只在刚好超出的那个字上
        let overflowing = match (ui.rect(key.child("line")), ui.rect(key.child("view"))) {
            (Some(line), Some(view)) => line.2 > view.2,
            _ => false,
        };
        let at_end = self.edit.caret() == self.edit.text().len();
        let view = layout::view(ui, multiline, content, overflowing, at_end)?;
        input::wire_line(ui, content, key.child("line"));
        input::wire_view(
            ui,
            view,
            key,
            self.disabled,
            self.edit.text().len(),
            &on_edit,
        );
        let node = paint::frame(ui, theme, &self, state, focused, multiline, key, view)?;
        input::wire(
            ui, key, &self, multiline, on_edit, on_commit, on_cancel, node,
        );
        Ok(node)
    }

    pub(super) fn style(&self, theme: &WidgetTheme, color: Color) -> TextStyle {
        theme.text(theme.font.size - 1.0, 400, color)
    }
}
