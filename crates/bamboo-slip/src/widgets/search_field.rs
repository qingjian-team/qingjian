//! 搜索框：放大镜加输入框，浅底无边框、撑满父节点的宽度。编辑同 [`TextField`]，状态是使用方保存的 [`TextEdit`]。

use std::hash::Hash;
use std::sync::Arc;

use taffy::NodeId;

use super::WidgetTheme;
use super::text_field::{Edit, TextEdit, TextField};
use crate::error::RenderError;
use crate::scene::Icon;
use crate::ui::Ui;

pub struct SearchField<'a> {
    edit: &'a TextEdit,

    placeholder: &'a str,

    icon: Option<Arc<dyn Icon>>,
}

impl<'a> SearchField<'a> {
    pub fn new(edit: &'a TextEdit) -> Self {
        Self {
            edit,
            placeholder: "",
            icon: None,
        }
    }

    pub fn placeholder(self, placeholder: &'a str) -> Self {
        Self {
            placeholder,
            ..self
        }
    }

    pub fn icon(self, icon: Arc<dyn Icon>) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    /// 参数同 [`TextField::build`]：回车产出 `on_commit`（跳到第一条结果这类），Esc 产出 `on_cancel`（清空）。
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_edit: impl Fn(Edit) -> M + Clone + 'static,
        on_commit: Option<M>,
        on_cancel: Option<M>,
    ) -> Result<NodeId, RenderError> {
        let mut field = TextField::new(self.edit)
            .placeholder(self.placeholder)
            .filled()
            .stretch();
        if let Some(icon) = self.icon {
            field = field.icon(icon);
        }
        field.build(ui, theme, local, on_edit, on_commit, on_cancel)
    }
}
