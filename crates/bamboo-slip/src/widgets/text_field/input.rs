//! 输入框的交互接线：内容行与视口的点击落点、打字、按键、失焦提交。

use taffy::NodeId;

use super::{Edit, TextField};
use crate::ui::{Interact, Key, KeyCode, Ui};

/// 内容行本身接一个空交互：点在文字之间的空白处也归这一行。
pub(super) fn wire_line<M>(ui: &mut Ui<M>, content: NodeId, key: Key) {
    ui.interact(content, key, Interact::new());
}

/// 视口拖到哪都把光标挪到末尾；禁用态不接。
pub(super) fn wire_view<M: Clone + 'static>(
    ui: &mut Ui<M>,
    view: NodeId,
    key: Key,
    disabled: bool,
    end: usize,
    on_edit: &(impl Fn(Edit) -> M + Clone + 'static),
) {
    let mut interact = Interact::new();
    if !disabled {
        let on_edit = on_edit.clone();
        interact = interact.on_drag(move |_| Some(on_edit(Edit::Caret(end))));
    }
    ui.interact(view, key.child("view"), interact);
}

/// 外框交互：密文、自动聚焦、收字、按键（回车/方向键/退格/复制剪切）、失焦提交。
pub(super) fn wire<M: Clone + 'static>(
    ui: &mut Ui<M>,
    key: Key,
    field: &TextField,
    multiline: bool,
    on_edit: impl Fn(Edit) -> M + Clone + 'static,
    on_commit: Option<M>,
    on_cancel: Option<M>,
    node: NodeId,
) {
    let secret = field.secret;
    let editing = on_edit.clone();
    let commit = on_commit.clone();
    let mut interact = Interact::new()
        .secret(secret)
        .autofocus(field.autofocus)
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
                    if command && (text == "a" || (!secret && (text == "c" || text == "x"))) =>
                {
                    Some(editing(Edit::Key(press.clone())))
                }
                _ => None,
            }
        })
        .disabled(field.disabled);
    if let Some(commit) = on_commit {
        interact = interact.on_blur(commit);
    }
    ui.interact(node, key, interact);
}
