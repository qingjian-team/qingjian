//! `annotation` 节点 `tones` 里的片段种类名，对应 [`Tone`]：`gloss` / `fresh` / `faint` / `pos` / `separator` / `code`。

use serde::Deserialize;

use crate::frame::Tone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ToneFilter {
    Gloss,

    Fresh,

    Faint,

    Pos,

    Separator,

    Code,
}

impl ToneFilter {
    pub(crate) fn matches(self, tone: Tone) -> bool {
        matches!(
            (self, tone),
            (Self::Gloss, Tone::Gloss)
                | (Self::Fresh, Tone::Fresh)
                | (Self::Faint, Tone::Faint)
                | (Self::Pos, Tone::Pos)
                | (Self::Separator, Tone::Separator)
                | (Self::Code, Tone::Code)
        )
    }
}
