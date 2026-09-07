use crate::engine::{MarkedKind, MarkedSegment};
use crate::parser::Segmentation;

use super::edit::Edit;

/// 一次拼写纠正：用户敲的串、纠正后的串、那一处编辑，以及纠正后串的完整音节切分。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    /// 用户敲的（作用域里的拼音，不含 `'`）。
    pub original: String,

    /// 纠正后的拼音。
    pub corrected: String,

    /// 从原串到纠正后串的那一处编辑。
    pub edit: Edit,

    /// 纠正后串的切分，每个音节都完整。
    pub segmentation: Segmentation,
}

impl Correction {
    /// 这处编辑落在纠正后哪个音节里：返回 (敲的那段字母, 纠正后的音节)，给个人敲错表记（敲的那段多半不是合法音节）；
    /// 多敲的字母算在它前面那个音节上（与 [`Edit::to_original`] 一致）。编辑处不在任何音节里时返回 `None`。
    /// `consumed` 是上屏消耗掉的纠正后字母数：没吃到编辑处的上屏不算接受了纠正。
    pub fn typo_pair(&self, consumed: usize) -> Option<(String, String)> {
        let at = match self.edit {
            Edit::Substitute { index, .. }
            | Edit::Insert { index }
            | Edit::Transpose { index, .. } => index,
            Edit::Delete { index, .. } => index.saturating_sub(1),
        };
        if at >= consumed {
            return None;
        }
        let mut start = 0;
        for syllable in &self.segmentation.syllables {
            let end = start + syllable.text.len();
            if at < end {
                let typed_start = self.edit.to_original(start);
                let typed_end = self.edit.to_original(end);
                let typed = self.original.get(typed_start..typed_end)?;
                return (!typed.is_empty() && typed != syllable.text)
                    .then(|| (typed.to_owned(), syllable.text.clone()));
            }
            start = end;
        }
        None
    }

    /// preedit 的分段：纠正后的切分拼音（`'` 连接），被改掉的原字母以 [`MarkedKind::Corrected`] 插在它原来的位置。
    /// `nihooma` → `ni'h` + ~~o~~ + `ao'ma`。
    pub fn marked_segments(&self) -> Vec<MarkedSegment> {
        let display = self.segmentation.joined("'");
        let Some((at, struck)) = self.edit.struck() else {
            return vec![MarkedSegment::new(display, MarkedKind::Typed)];
        };
        // 纠正后串的字母下标 → 显示串（含 `'`）的下标
        let mut letters = 0;
        let mut split = display.len();
        for (i, c) in display.char_indices() {
            if c == '\'' {
                continue;
            }
            if letters == at {
                split = i;
                break;
            }
            letters += 1;
        }
        let mut segments = Vec::with_capacity(3);
        if split > 0 {
            segments.push(MarkedSegment::new(&display[..split], MarkedKind::Typed));
        }
        segments.push(MarkedSegment::new(struck, MarkedKind::Corrected));
        if split < display.len() {
            segments.push(MarkedSegment::new(&display[split..], MarkedKind::Typed));
        }
        segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Syllable;

    fn segmentation(syllables: &[&str]) -> Segmentation {
        Segmentation {
            syllables: syllables.iter().map(|s| Syllable::complete(s)).collect(),
        }
    }

    fn texts(segments: &[MarkedSegment]) -> Vec<(String, MarkedKind)> {
        segments.iter().map(|s| (s.text.clone(), s.kind)).collect()
    }

    #[test]
    fn strikes_the_replaced_letter_in_place() {
        let correction = Correction {
            original: "nihooma".into(),
            corrected: "nihaoma".into(),
            edit: Edit::Substitute {
                index: 3,
                from: 'o',
            },
            segmentation: segmentation(&["ni", "hao", "ma"]),
        };
        assert_eq!(
            texts(&correction.marked_segments()),
            [
                ("ni'h".to_owned(), MarkedKind::Typed),
                ("o".to_owned(), MarkedKind::Corrected),
                ("ao'ma".to_owned(), MarkedKind::Typed),
            ]
        );
    }

    #[test]
    fn typo_pair_is_the_syllable_around_the_edit() {
        let correction = Correction {
            original: "nihooma".into(),
            corrected: "nihaoma".into(),
            edit: Edit::Substitute {
                index: 3,
                from: 'o',
            },
            segmentation: segmentation(&["ni", "hao", "ma"]),
        };
        assert_eq!(
            correction.typo_pair(7),
            Some(("hoo".to_owned(), "hao".to_owned()))
        );
        // 只吃到 ni 就上屏：没接受纠正
        assert_eq!(correction.typo_pair(2), None);
        let correction = Correction {
            original: "meiganxi".into(),
            corrected: "meiguanxi".into(),
            edit: Edit::Insert { index: 4 },
            segmentation: segmentation(&["mei", "guan", "xi"]),
        };
        assert_eq!(
            correction.typo_pair(9),
            Some(("gan".to_owned(), "guan".to_owned()))
        );
        let correction = Correction {
            original: "zhegge".into(),
            corrected: "zhege".into(),
            edit: Edit::Delete {
                index: 3,
                removed: 'g',
            },
            segmentation: segmentation(&["zhe", "ge"]),
        };
        // 多敲的 g 算在前一个音节上
        assert_eq!(
            correction.typo_pair(5),
            Some(("zheg".to_owned(), "zhe".to_owned()))
        );
    }

    #[test]
    fn extra_letter_at_the_end_and_missing_letter() {
        let correction = Correction {
            original: "nihaox".into(),
            corrected: "nihao".into(),
            edit: Edit::Delete {
                index: 5,
                removed: 'x',
            },
            segmentation: segmentation(&["ni", "hao"]),
        };
        assert_eq!(
            texts(&correction.marked_segments()),
            [
                ("ni'hao".to_owned(), MarkedKind::Typed),
                ("x".to_owned(), MarkedKind::Corrected),
            ]
        );
        let correction = Correction {
            original: "nhao".into(),
            corrected: "nihao".into(),
            edit: Edit::Insert { index: 1 },
            segmentation: segmentation(&["ni", "hao"]),
        };
        assert_eq!(
            texts(&correction.marked_segments()),
            [("ni'hao".to_owned(), MarkedKind::Typed)]
        );
    }
}
