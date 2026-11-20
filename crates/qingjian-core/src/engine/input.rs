//! Engine 的会话输入状态（本次组句内的缓冲区附属状态）。

use std::time::Instant;

use super::held::HeldSelection;
use crate::candidate::Language;
use crate::composition::Composition;

/// 一次组句内的输入状态：待上屏的选词、直通字符、翻页计数、起始时刻、退格痕迹与候选窗译词。
///
/// 这些字段都跟着「当前这一段输入」走，换上下文时与 `EngineSession` 里的同名字段交换。
#[derive(Default)]
pub(crate) struct EngineInput {
    /// 当前拼音缓冲区。
    pub(crate) composition: Composition,

    /// 英文模式（壳里 Caps Lock 亮着）：缓冲区里的字母不当拼音，候选来自英文词表的补全与纠正。
    pub(crate) english_mode: bool,

    /// 前面选好、还没交给应用的词（见 `held` 模块）。
    pub(crate) held: Vec<HeldSelection>,

    /// 组句外直通给应用的字符，攒到下一次上屏或上文断开时写成一条 `passthrough`。
    pub(crate) passthrough_pending: String,

    /// 这段组句翻了几页候选。
    pub(crate) page_turns: u32,

    /// 这段组句第一键的时刻（算首键到上屏的毫秒）。
    pub(crate) composition_started: Option<Instant>,

    /// 上一记退格是不是正好把组句删空了：下一记如果是自动重复就吞掉，
    /// 别让它接着删应用里已经上屏的字。
    pub(crate) backspace_emptied: bool,

    /// 这段组句里第一次退格前的缓冲区：上屏时与最终键串不同就记一条 `retype`。
    pub(crate) retype_snapshot: Option<String>,

    /// 候选窗口当前页上的译词（壳每次画完告知），上屏时记成「看到过」。
    pub(crate) displayed: Vec<(Language, String)>,
}
