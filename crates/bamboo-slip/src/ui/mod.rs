//! 交互层：每次状态变了重建整棵树，元素按稳定 [`Key`] 跨帧认回悬停、按下、焦点；输入事件命中元素后冒泡，处理结果是使用方定义的消息。
//!
//! 一帧的流程：
//! 1. [`Ui::new`] 借着 [`Interaction`] 建树，控件用 [`Ui::state`] 取自己的状态挑样式，用 [`Ui::interact`] 登记怎么响应；
//! 2. [`Ui::finish`] 交出场景与登记表，场景布局之后交给 [`Interaction::attach`]，再照常交给 `Stage` 画；
//! 3. 宿主把输入翻成 [`Event`] 交给 [`Interaction::handle`]，按返回的 [`Response`] 更新应用状态、需要时回到第 1 步。
//!
//! 命中只看布局给的矩形：过渡、循环动画中途的位置不算。

mod builder;
mod element;
mod elements;
mod event;
mod interact;
mod interaction;
mod key;
mod response;
mod scroll_extent;
mod searchable;
mod state;

pub use builder::Ui;
pub use elements::Elements;
pub use event::{Event, KeyCode, KeyPress, Modifiers, TextInput};
pub use interact::Interact;
pub use interaction::Interaction;
pub use key::Key;
pub use response::Response;
pub use scroll_extent::ScrollExtent;
pub use searchable::Searchable;
pub use state::ElementState;
