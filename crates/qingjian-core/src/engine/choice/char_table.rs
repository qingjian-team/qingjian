//! 选词模型按音节出的字分布（知意的 composition 头）：每个音节一行，各字的 log 概率。整句词图拿它给路径上的字打分。

use std::collections::HashMap;
use std::sync::Arc;

/// 一次输入的字分布。
#[derive(Debug, Clone)]
pub struct CharTable {
    /// 字 → 列号，同一个模型的所有表共用。
    pub index: Arc<HashMap<char, u32>>,

    /// 每个音节一行，长度与模型字表相同。
    rows: Vec<Vec<f32>>,

    /// 每行的最大值：词图按「比这个位置最好的字差多少」打分，各位置一样公平。
    best: Vec<f32>,
}

impl CharTable {
    pub fn new(index: Arc<HashMap<char, u32>>, rows: Vec<Vec<f32>>) -> Self {
        let best = rows
            .iter()
            .map(|row| row.iter().copied().fold(f32::NEG_INFINITY, f32::max))
            .collect();
        Self { index, rows, best }
    }

    /// 第 `slot` 个音节读成 `ch` 比这个位置最好的字差多少（≤ 0）。
    pub fn relative(&self, slot: usize, ch: char) -> Option<f32> {
        Some(self.log_prob(slot, ch)? - *self.best.get(slot)?)
    }

    /// 第 `slot` 个音节读成 `ch` 的 log 概率；越界或字表里没有这个字返回 `None`。
    pub fn log_prob(&self, slot: usize, ch: char) -> Option<f32> {
        let column = *self.index.get(&ch)? as usize;
        self.rows.get(slot)?.get(column).copied()
    }
}
