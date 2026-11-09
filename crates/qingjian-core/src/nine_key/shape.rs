//! 九宫格数字串的一种切法。

use qingjian_dictionary::SyllablePattern;

use super::Position;

/// 一种切法：边界定了，每个位置的读法还没定。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    pub positions: Vec<Position>,
}

impl Shape {
    pub(super) fn extended(&self, position: Position) -> Self {
        let mut positions = self.positions.clone();
        positions.push(position);
        Self { positions }
    }

    /// 与全拼切分同一个排序键：音节少 > 不完整音节少 > 前面的音节长。
    pub(super) fn sort_key(&self) -> (usize, usize, Vec<std::cmp::Reverse<usize>>) {
        (
            self.positions.len(),
            self.positions.iter().filter(|p| !p.complete).count(),
            self.positions
                .iter()
                .map(|p| std::cmp::Reverse(p.len))
                .collect(),
        )
    }

    /// 覆盖的输入字节数（含中间的 `'`）。
    pub fn end(&self) -> usize {
        self.positions.last().map_or(0, |p| p.start + p.len)
    }

    /// 第一个还没定下来的位置（写法不止一种）：拼音列列的、选定音节时换掉的都是它。
    pub fn first_open(&self) -> Option<(usize, &Position)> {
        self.positions
            .iter()
            .enumerate()
            .find(|(_, p)| p.alternatives.len() > 1)
    }

    /// 前 `count` 个位置的多写法模式。
    pub fn patterns(&self, count: usize) -> Vec<Vec<SyllablePattern<'_>>> {
        self.positions[..count]
            .iter()
            .map(Position::patterns)
            .collect()
    }

    /// 非末尾的简拼位置数，见 `ranking::Scored::abbreviated`。
    pub fn abbreviated(&self, count: usize) -> usize {
        self.positions[..count]
            .iter()
            .rev()
            .skip(1)
            .filter(|p| !p.complete)
            .count()
    }
}
