//! 候选旁边那行小字的**分段着色**：Core 编成一段字符串，壳解码成多段、按种类上色。
//!
//! 有些注记里个别位置要突出：`u` 模式笔画打错位时，正确的笔顺里错的那一位要标出来；
//! 以后还会有别的（部件高亮、命中片段、纠错提示…）。与其每加一处就给 [`crate::Candidate`]
//! 添一个字段（那要改上百处字面量与四个壳），不如约定一段可以带分段标记的文本。
//!
//! 每段前面写一个标记字符（Unicode 私用区，正常文本里不会出现），解码时按标记切段、
//! 把标记剥掉。标记是「段的开始」而不是「切换」，所以任何一段跟着另一段都不用另写结束符。
//!
//! ```no_run
//! use qingjian_core::annotation::{encode, Note};
//!
//! // `hpzhhsn` 里第二笔打错了：只把那位标出来
//! let text = encode(&[Note::plain("h"), Note::emphasis("p"), Note::plain("zhhsn")]);
//! ```

/// 普通段的起始标记。
pub const PLAIN: char = '\u{e000}';

/// 强调段的起始标记。
pub const EMPHASIS: char = '\u{e001}';

/// 弱化段的起始标记。
pub const MUTED: char = '\u{e002}';

/// 一段注记的种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    /// 普通文字。
    Plain,

    /// 要突出的（笔画里打错的那一位、命中的那个部件）。
    Emphasis,

    /// 更淡的附加说明（假名注音之类）。
    Muted,
}

impl NoteKind {
    fn mark(self) -> char {
        match self {
            Self::Plain => PLAIN,
            Self::Emphasis => EMPHASIS,
            Self::Muted => MUTED,
        }
    }
}

/// 注记里的一段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub text: String,
    pub kind: NoteKind,
}

impl Note {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: NoteKind::Plain,
        }
    }

    pub fn emphasis(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: NoteKind::Emphasis,
        }
    }

    pub fn muted(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: NoteKind::Muted,
        }
    }
}

/// 把分段拼成一段可交给壳的文本：每段前面写它的标记。
pub fn encode(notes: &[Note]) -> String {
    let mut out = String::new();
    for note in notes.iter().filter(|note| !note.text.is_empty()) {
        out.push(note.kind.mark());
        out.push_str(&note.text);
    }
    out
}

/// 拆回分段。不带标记的文本整段算普通段（老数据、普通译词都是这种）。
pub fn decode(text: &str) -> Vec<(String, NoteKind)> {
    let mut out: Vec<(String, NoteKind)> = Vec::new();
    let mut current = String::new();
    let mut kind = NoteKind::Plain;
    for ch in text.chars() {
        let marked = match ch {
            PLAIN => Some(NoteKind::Plain),
            EMPHASIS => Some(NoteKind::Emphasis),
            MUTED => Some(NoteKind::Muted),
            _ => None,
        };
        match marked {
            Some(next) => {
                if !current.is_empty() {
                    out.push((std::mem::take(&mut current), kind));
                }
                kind = next;
            }
            None => current.push(ch),
        }
    }
    if !current.is_empty() {
        out.push((current, kind));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_round_trip() {
        let text = encode(&[
            Note::plain("h"),
            Note::emphasis("p"),
            Note::plain("zh"),
            Note::muted("hsn"),
        ]);
        assert_eq!(
            decode(&text),
            vec![
                ("h".to_owned(), NoteKind::Plain),
                ("p".to_owned(), NoteKind::Emphasis),
                ("zh".to_owned(), NoteKind::Plain),
                ("hsn".to_owned(), NoteKind::Muted),
            ]
        );
    }

    #[test]
    fn plain_text_is_one_segment() {
        assert_eq!(decode("lín"), vec![("lín".to_owned(), NoteKind::Plain)]);
        assert!(decode("").is_empty());
    }

    /// 相邻两段同类也要分得开：标记是「段的开始」，不是「切换」。
    #[test]
    fn same_kind_neighbours_stay_separate() {
        let text = encode(&[Note::emphasis("p"), Note::emphasis("z")]);
        assert_eq!(
            decode(&text),
            vec![
                ("p".to_owned(), NoteKind::Emphasis),
                ("z".to_owned(), NoteKind::Emphasis),
            ]
        );
    }
}
