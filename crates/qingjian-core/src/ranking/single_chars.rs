//! 单字模式（#848）：组句中按快捷键后候选只留单字，方便第一页找不到时逐字选。

use crate::candidate::{Candidate, CandidateKind};

/// 只留词库里的单字（拼音与形码的都算），词、整句、英文、emoji 与快捷候选都拿掉，原来的先后不变。
/// 一个单字都没有时（表达式、问字、英文直输段）不动，免得候选窗变空。
pub fn keep_only_single_chars(items: &mut Vec<Candidate>) {
    let is_single = |c: &Candidate| {
        matches!(c.kind, CandidateKind::Chinese | CandidateKind::Code)
            && c.text.chars().count() == 1
    };
    if items.iter().any(is_single) {
        items.retain(is_single);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(text: &str, kind: CandidateKind) -> Candidate {
        Candidate {
            text: text.to_owned(),
            kind,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        }
    }

    #[test]
    fn keeps_single_chars_in_order() {
        let mut items = vec![
            candidate("你好", CandidateKind::Sentence),
            candidate("你", CandidateKind::Chinese),
            candidate("hello", CandidateKind::English),
            candidate("拟好", CandidateKind::Chinese),
            candidate("😀", CandidateKind::Emoji),
            candidate("尼", CandidateKind::Chinese),
        ];
        keep_only_single_chars(&mut items);
        let texts: Vec<&str> = items.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(texts, ["你", "尼"]);
    }

    #[test]
    fn leaves_lists_without_single_chars_alone() {
        let mut items = vec![
            candidate("1+1=2", CandidateKind::Shortcut),
            candidate("hello", CandidateKind::English),
        ];
        keep_only_single_chars(&mut items);
        assert_eq!(items.len(), 2);
    }
}
