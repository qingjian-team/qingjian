//! Engine 里的查询缓存与上一次查询的痕迹。

use std::cell::{Cell, RefCell};

use super::choice;
use super::input_log;
use super::query;
use super::rescoring;
use super::sentence;

/// Engine 里所有「跨按键留着」的格子：查询结果的缓存、上一次查询的摘要与痕迹。
///
/// 单独放一块，Engine 的字段表里不再混着九个 `RefCell` / `Cell`。
#[derive(Default)]
pub(crate) struct EngineCaches {
    /// 「前文 + 整句文本 → 神经分」缓存，同步与异步打分共用。
    pub(crate) neural: RefCell<rescoring::NeuralCache>,

    /// 最近一次查询的候选顺序是否经过神经重排（`rescore_paths` 置位，`query` 开头清零），写进输入日志。
    pub(crate) last_rescored: Cell<bool>,

    /// 最近一次查询里选词模型排过这页的记录（`rerank_choices` 写，`query` 开头清零），写进输入日志。
    pub(crate) last_choice: Cell<Option<input_log::ChoiceNote>>,

    /// 最近一次查询里选词模型给各候选的分（`rerank_choices` 写，`query` 开头清空），给评测看。
    pub(crate) last_choice_scores: RefCell<Vec<choice::ChoiceScore>>,

    /// 这次查询里敲错边读出、当备选插进来的整句（`Engine::plain_sentence`）：选词重排不拿它们比。
    pub(crate) corrected_alternates: RefCell<Vec<String>>,

    /// 上一次查询的摘要，上屏时写进输入日志。
    pub(crate) last_query: RefCell<Option<query::QuerySnapshot>>,

    /// 这段组句生成过的 UUID / 时间戳：(关键字, 各写法)。壳每次刷新都重查候选，同一段组句里得是同一组值；新一段组句的第一键清掉。
    pub(crate) generated_forms: RefCell<Option<(String, Vec<String>)>>,

    /// 上一次算过的拼写纠正：(作用域, 结果)。query 算一次，commit / take_raw 复用，别再跑一遍变体枚举。
    pub(crate) correction: RefCell<Option<(String, Option<crate::correction::Correction>)>>,

    /// 整句转换的格子候选缓存：跨按键复用，学习数据一变就清（见 `Engine::forget_span_cache`）。
    pub(crate) span: RefCell<sentence::SpanCache>,
}
