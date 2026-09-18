//! annotation 片段的深浅。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 译文。
    Gloss,

    /// 生词的译文（用户还没在候选里见过几轮），用强调色。
    Fresh,

    /// 词性与分隔符，最浅。
    Faint,

    /// 辅码态命中的那条码（`[general] aux_code_show` 打开时才有）：与译文同一个淡色。
    Code,
}
