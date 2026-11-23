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

#[cfg(test)]
mod tests {
    use super::ToneFilter;
    use crate::frame::Tone;

    fn every_pair() -> [(ToneFilter, Tone); 6] {
        [
            (ToneFilter::Gloss, Tone::Gloss),
            (ToneFilter::Fresh, Tone::Fresh),
            (ToneFilter::Faint, Tone::Faint),
            (ToneFilter::Pos, Tone::Pos),
            (ToneFilter::Separator, Tone::Separator),
            (ToneFilter::Code, Tone::Code),
        ]
    }

    #[test]
    fn every_tone_name_parses() {
        for (json, expected) in [
            ("\"gloss\"", ToneFilter::Gloss),
            ("\"fresh\"", ToneFilter::Fresh),
            ("\"faint\"", ToneFilter::Faint),
            ("\"pos\"", ToneFilter::Pos),
            ("\"separator\"", ToneFilter::Separator),
            ("\"code\"", ToneFilter::Code),
        ] {
            assert_eq!(serde_json::from_str::<ToneFilter>(json).unwrap(), expected);
        }
        assert!(serde_json::from_str::<ToneFilter>("\"unknown\"").is_err());
        assert!(serde_json::from_str::<ToneFilter>("\"Gloss\"").is_err());
    }

    #[test]
    fn a_filter_takes_only_its_own_tone() {
        for (filter, tone) in every_pair() {
            assert!(filter.matches(tone), "{filter:?} 应吃掉 {tone:?}");
            for (other, other_tone) in every_pair() {
                if other != filter {
                    assert!(
                        !filter.matches(other_tone),
                        "{filter:?} 不该吃 {other_tone:?}"
                    );
                }
            }
        }
    }
}
