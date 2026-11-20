//! 一次退格的处理结果（[`Backspace`]），由 [`Engine::backspace`](super::Engine::backspace) 返回。

/// 一次退格的处理结果：壳据此决定这个键是吞掉还是交给应用。
///
/// 组句删空之后的自动重复用 [`Backspace::Held`] 停在输入法里，不接着删应用里已经上屏的字。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backspace {
    /// 删掉了组句里的东西（拼音 / 选好的词 / 码段），这个键吞掉。
    Deleted,

    /// 组句已经空着，交给应用删字。
    Released,

    /// 刚把组句删空，这一记自动重复先吞掉——手一直按就一直不往下传，松手再按才删。
    Held,
}

impl Backspace {
    /// 这个键有没有留在输入法里（`Held` 也算，它没交给应用）。
    pub fn consumed(self) -> bool {
        !matches!(self, Self::Released)
    }
}
