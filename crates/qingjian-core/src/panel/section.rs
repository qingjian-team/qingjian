//! 面板的一个分类。

use super::PanelGroup;
use crate::candidate::{Candidate, CandidateKind};

/// 面板一行最多几格：符号与 emoji 一格一字，十格一行。
pub const PANEL_COLUMNS: usize = 10;

/// 颜文字这类宽条目少放几格（面板总宽不变，格子拉宽）。
const MEDIUM_COLUMNS: usize = 6;
const WIDE_COLUMNS: usize = 4;

/// 估宽时一条最多算几个字宽：再宽的在格子里截断，信息行里看全文。
const MAX_ITEM_EMS: f32 = 8.0;

/// 不超过这么多字宽的条目一格放得下，分类按满格排。
pub(crate) const NARROW_EMS: f32 = 2.2;

/// 一个分类：所在分组、名字与条目。条目是现成的候选（种类是 emoji / 颜文字 / 符号，`reading` 是中文名字，可能没有），
/// 壳拿去排格子、上屏走 [`crate::Engine::commit`]。
#[derive(Debug, Clone)]
pub struct PanelSection {
    pub group: PanelGroup,

    pub name: String,

    pub items: Vec<Candidate>,
}

impl PanelSection {
    /// 一行几格，按分类里最宽的条目定。
    pub fn columns(&self) -> usize {
        let ems = self.widest_ems();
        if ems <= NARROW_EMS {
            PANEL_COLUMNS
        } else if ems <= 5.0 {
            MEDIUM_COLUMNS
        } else {
            WIDE_COLUMNS
        }
    }

    /// 最宽的一条按几个字宽估（[`item_ems`]）。
    fn widest_ems(&self) -> f32 {
        self.items
            .iter()
            .map(item_ems)
            .fold(1.0_f32, f32::max)
            .min(MAX_ITEM_EMS)
    }
}

/// 一条按几个字宽估：emoji 一个字宽，ASCII 半个多，组合附加符号与变体选择符不占宽。
pub(crate) fn item_ems(item: &Candidate) -> f32 {
    if item.kind == CandidateKind::Emoji {
        return 1.0;
    }
    item.text
        .chars()
        .map(|c| match c {
            '\u{0300}'..='\u{036F}' | '\u{FE00}'..='\u{FE0F}' => 0.0,
            c if c.is_ascii() => 0.6,
            _ => 1.0,
        })
        .sum()
}
