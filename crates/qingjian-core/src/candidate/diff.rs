//! 长候选里和首选不一样的字：几个长候选看着差不多时，候选窗把这些字画成另一种颜色，一眼看出差在哪。

use std::ops::Range;

use super::{Candidate, CandidateKind};

/// 首选至少这么多字才比：两三个字的词一眼就能看全，标出来反而花。
const MIN_CHARS: usize = 4;

/// `other` 里和首选 `lead` 不一样的字，按 `other.text` 的字节范围给出，相邻的连成一段。
///
/// 只比字数相同的两条，逐字对位置：首选覆盖整段拼音，字数相同的候选也读完了整段。不看两条带的音节：
/// 学过的整句、云端给的候选与本地整句的音节记法不一定一样，读的仍是同一串拼音。
/// 首选太短、两条一样、或者一个字都对不上时（整条都不一样，标了等于没标）给空。
pub fn differs_from_lead(lead: &Candidate, other: &Candidate) -> Vec<Range<usize>> {
    if !comparable(lead) || !comparable(other) {
        return Vec::new();
    }
    let lead_chars: Vec<char> = lead.text.chars().collect();
    if lead_chars.len() < MIN_CHARS || other.text.chars().count() != lead_chars.len() {
        return Vec::new();
    }
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut same = 0;
    for ((byte, ch), lead_ch) in other.text.char_indices().zip(&lead_chars) {
        if ch == *lead_ch {
            same += 1;
            continue;
        }
        let end = byte + ch.len_utf8();
        match ranges.last_mut() {
            Some(last) if last.end == byte => last.end = end,
            _ => ranges.push(byte..end),
        }
    }
    if same == 0 {
        return Vec::new();
    }
    ranges
}

/// 拼音打出来的中文候选（含模型按按键直接生成的整句）；表情、符号、英文、快捷候选和形码不比。
fn comparable(candidate: &Candidate) -> bool {
    matches!(
        candidate.kind,
        CandidateKind::Chinese
            | CandidateKind::Cloud
            | CandidateKind::Sentence
            | CandidateKind::Generated
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(text: &str, syllables: &str) -> Candidate {
        Candidate {
            text: text.to_owned(),
            kind: CandidateKind::Sentence,
            syllables: syllables.split(' ').map(str::to_owned).collect(),
            reading: None,
            translation: None,
            aux_code: None,
        }
    }

    fn marked(lead: &Candidate, other: &Candidate) -> Vec<String> {
        differs_from_lead(lead, other)
            .into_iter()
            .map(|range| other.text[range].to_owned())
            .collect()
    }

    #[test]
    fn marks_the_characters_that_differ() {
        let lead = candidate(
            "实现一边输入中文证据",
            "shi xian yi bian shu ru zhong wen zheng ju",
        );
        let other = candidate(
            "实现一边输入中文整句",
            "shi xian yi bian shu ru zhong wen zheng ju",
        );
        assert_eq!(marked(&lead, &other), ["整句"]);
        let split = candidate(
            "事先一边输入中文证据",
            "shi xian yi bian shu ru zhong wen zheng ju",
        );
        assert_eq!(marked(&lead, &split), ["事先"]);
        let two = candidate(
            "事先一边输入中文整句",
            "shi xian yi bian shu ru zhong wen zheng ju",
        );
        assert_eq!(marked(&lead, &two), ["事先", "整句"]);
    }

    #[test]
    fn the_lead_itself_has_nothing_marked() {
        let lead = candidate("今天天气不错", "jin tian tian qi bu cuo");
        assert!(differs_from_lead(&lead, &lead).is_empty());
    }

    #[test]
    fn short_or_misaligned_candidates_are_not_compared() {
        // 首选不到四个字
        let lead = candidate("实现", "shi xian");
        assert!(differs_from_lead(&lead, &candidate("事先", "shi xian")).is_empty());
        // 只读了前半段拼音的词
        let lead = candidate("实现一边输入", "shi xian yi bian shu ru");
        assert!(differs_from_lead(&lead, &candidate("实现", "shi xian")).is_empty());
        // 字数对不上（带了英文字母）
        let lead = candidate("今天天气不错", "jin tian tian qi bu cuo");
        let other = candidate("今天天气bu错", "jin tian tian qi bu cuo");
        assert!(differs_from_lead(&lead, &other).is_empty());
    }

    /// 学过的整句、云端候选带的音节与本地整句不完全一样，照样逐字比。
    #[test]
    fn syllables_recorded_differently_are_still_compared() {
        let lead = candidate(
            "实现一边输入中文证据",
            "shi xian yi bian shu ru zhong wen zheng ju",
        );
        let learned = candidate(
            "实现一遍输入中文证据",
            "shi xian yi bian shu ru zhong wen zhengju",
        );
        assert_eq!(marked(&lead, &learned), ["遍"]);
        let mut generated = candidate("实现以便输入中文证据", "");
        generated.kind = CandidateKind::Generated;
        assert_eq!(marked(&lead, &generated), ["以便"]);
        let mut cloud = candidate("实现一边输入中文政据", "");
        cloud.kind = CandidateKind::Cloud;
        assert_eq!(marked(&lead, &cloud), ["政"]);
    }

    #[test]
    fn nothing_in_common_marks_nothing() {
        let lead = candidate("实现一边", "shi xian yi bian");
        let other = candidate("时限异变", "shi xian yi bian");
        assert!(differs_from_lead(&lead, &other).is_empty());
    }

    #[test]
    fn emoji_and_english_are_not_compared() {
        let lead = candidate("今天天气不错", "jin tian tian qi bu cuo");
        let mut other = candidate("今天天气不措", "jin tian tian qi bu cuo");
        other.kind = CandidateKind::Emoji;
        assert!(differs_from_lead(&lead, &other).is_empty());
    }
}
