//! 候选全部插完（整句、英文、emoji）、选词模型重排并截完整句之后的收尾：首音节单字至少有一个留在第一页。

use crate::candidate::{Candidate, CandidateKind};

/// 首音节的第一个单字最晚排到第几位（从 1 数）：缺省也是最大的一页 9 个。
/// [`super::arrange`] 只给词限了名额，整句、emoji 后插进来不算名额，长拼音时会把单字挤到第二页（#769）。
pub const FIRST_SINGLE_CHAR_BY: usize = 9;

/// 输入不止一个音节、首音节的第一个单字排在 [`FIRST_SINGLE_CHAR_BY`] 之后时，把单字前面离它最近的词
/// （连同紧跟着配给它的 emoji、颜文字、符号）挪到单字那一段后面，挪到单字进第一页或前面没有词可挪为止。
/// 整句、英文、快捷候选不动，这个输入下选过的词（`kept`）也不动，与 [`super::arrange`] 一样；挪走的词保持原来的先后。
pub fn keep_single_char_on_first_page(
    items: &mut Vec<Candidate>,
    several: bool,
    kept: impl Fn(&Candidate) -> bool,
) {
    if !several {
        return;
    }
    let is_single = |c: &Candidate| c.kind == CandidateKind::Chinese && c.text.chars().count() == 1;
    let Some(mut first) = items.iter().position(is_single) else {
        return;
    };
    let mut moved: Vec<Vec<Candidate>> = Vec::new();
    while first >= FIRST_SINGLE_CHAR_BY {
        let Some(word) = items[..first]
            .iter()
            .rposition(|c| c.kind == CandidateKind::Chinese && !kept(c))
        else {
            break;
        };
        let end = word
            + 1
            + items[word + 1..first]
                .iter()
                .take_while(|c| {
                    matches!(
                        c.kind,
                        CandidateKind::Emoji | CandidateKind::Kaomoji | CandidateKind::Symbol
                    )
                })
                .count();
        moved.push(items.drain(word..end).collect());
        first -= end - word;
    }
    if moved.is_empty() {
        return;
    }
    let after = first + items[first..].iter().take_while(|c| is_single(c)).count();
    let block: Vec<Candidate> = moved.into_iter().rev().flatten().collect();
    items.splice(after..after, block);
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

    fn list(items: &[(&str, CandidateKind)]) -> Vec<Candidate> {
        items.iter().map(|(t, k)| candidate(t, *k)).collect()
    }

    fn texts(items: &[Candidate]) -> Vec<&str> {
        items.iter().map(|c| c.text.as_str()).collect()
    }

    #[test]
    fn words_nearest_the_chars_make_room_on_the_first_page() {
        use CandidateKind::{Chinese, Emoji, Sentence};
        let mut items = list(&[
            ("句一", Sentence),
            ("句二", Sentence),
            ("句三", Sentence),
            ("实验室", Chinese),
            ("🔬", Emoji),
            ("🧫", Emoji),
            ("十堰市", Chinese),
            ("试验室", Chinese),
            ("实验师", Chinese),
            ("饰演", Chinese),
            ("实验", Chinese),
            ("是", Chinese),
            ("时", Chinese),
            ("实验员", Chinese),
        ]);
        keep_single_char_on_first_page(&mut items, true, |_| false);
        assert_eq!(
            texts(&items),
            [
                "句一",
                "句二",
                "句三",
                "实验室",
                "🔬",
                "🧫",
                "十堰市",
                "试验室",
                "是",
                "时",
                "实验师",
                "饰演",
                "实验",
                "实验员"
            ]
        );
    }

    #[test]
    fn emoji_move_with_their_word_and_sentences_stay() {
        use CandidateKind::{Chinese, Emoji, Sentence};
        let mut items = list(&[
            ("句一", Sentence),
            ("句二", Sentence),
            ("句三", Sentence),
            ("句四", Sentence),
            ("句五", Sentence),
            ("句六", Sentence),
            ("句七", Sentence),
            ("句八", Sentence),
            ("实验室", Chinese),
            ("🔬", Emoji),
            ("句九", Sentence),
            ("是", Chinese),
        ]);
        keep_single_char_on_first_page(&mut items, true, |_| false);
        assert_eq!(
            texts(&items),
            [
                "句一",
                "句二",
                "句三",
                "句四",
                "句五",
                "句六",
                "句七",
                "句八",
                "句九",
                "是",
                "实验室",
                "🔬"
            ]
        );
    }

    #[test]
    fn words_chosen_for_this_input_stay_put() {
        use CandidateKind::Chinese;
        let mut items: Vec<Candidate> = [
            "一一", "二二", "三三", "四四", "五五", "六六", "七七", "八八", "选过", "是",
        ]
        .into_iter()
        .map(|t| candidate(t, Chinese))
        .collect();
        keep_single_char_on_first_page(&mut items, true, |c| c.text == "选过");
        assert_eq!(texts(&items)[7..], ["选过", "是", "八八"]);
    }

    #[test]
    fn one_syllable_input_and_early_chars_are_left_alone() {
        use CandidateKind::Chinese;
        let words: Vec<(&str, CandidateKind)> = [
            "一一", "二二", "三三", "四四", "五五", "六六", "七七", "八八", "九九", "是",
        ]
        .into_iter()
        .map(|t| (t, Chinese))
        .collect();
        let mut items = list(&words);
        keep_single_char_on_first_page(&mut items, false, |_| false);
        assert_eq!(texts(&items).last(), Some(&"是"));
        let mut items = list(&words[1..]);
        keep_single_char_on_first_page(&mut items, true, |_| false);
        assert_eq!(texts(&items).last(), Some(&"是"));
    }
}
