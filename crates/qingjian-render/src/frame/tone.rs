//! annotation 片段的种类：决定颜色，主题也按它挑着画（`annotation` 节点的 `tones`）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 译文。
    Gloss,

    /// 生词的译文（用户还没在候选里见过几轮），用强调色。
    Fresh,

    /// 其余的淡色片段（日文译词的假名注音），最浅。
    Faint,

    /// 词性（`n.`、`int.`），缺省与 `Faint` 同色。
    Pos,

    /// 义项之间的分隔（` · `），缺省与 `Faint` 同色；主题按它数义项。
    Separator,

    /// 辅码态命中的那条码（`[general] aux_code_show` 打开时才有）：与译文同一个淡色。
    Code,
}
