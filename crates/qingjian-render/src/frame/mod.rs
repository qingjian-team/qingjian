//! 一帧要画的全部内容：顶部拼音行、候选行、高亮、页脚、右侧整句补全。只是展示形态，不含排序或查词。

mod preedit;
mod row;
mod tone;

pub use preedit::{Preedit, PreeditSegment, PreeditStyle};
pub use row::Row;
pub use tone::Tone;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frame {
    /// 顶部拼音行；配置成只在行内显示时为 `None`。
    pub preedit: Option<Preedit>,

    /// 候选行。
    pub rows: Vec<Row>,

    /// 高亮行；`None` 不高亮。
    pub highlighted: Option<usize>,

    /// 横排展开成矩阵时每行几格，`rows` 按行优先排开、空位是空行；0 为没展开。
    pub columns: usize,

    /// 右下角页码。
    pub footer: Option<String>,

    /// 拼音行右侧的整句补全（云联想），组句时才有。
    pub sentence: Option<String>,

    /// 拼音行右侧的一句临时状态（删了什么词），有它时不画整句补全。
    pub status: Option<String>,
}

impl Frame {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.preedit.is_none() && self.trailing().is_none()
    }

    /// 顶部要不要画一行（拼音或右侧文字任一存在）。
    pub fn has_top_line(&self) -> bool {
        self.preedit.is_some() || self.trailing().is_some()
    }

    /// 拼音行右侧画什么：状态优先，其次整句补全；`bool` 是要不要带云朵。
    pub fn trailing(&self) -> Option<(&str, bool)> {
        self.status
            .as_deref()
            .map(|s| (s, false))
            .or_else(|| self.sentence.as_deref().map(|s| (s, true)))
    }
}
