//! 编辑会话：TSF 不允许直接改文档，要经 `RequestEditSession` 申请、在回调里拿着 edit cookie 读写。
//! 写组句 / 上屏在 [`update`]，读选区（翻译选中文字）在 [`selection`]，候选窗口的定位锚点在 [`anchor`]。

mod anchor;
mod selection;
mod update;

pub(crate) use self::anchor::anchor_rect;
pub(crate) use self::selection::request_selection;
pub(crate) use self::update::request_update;
