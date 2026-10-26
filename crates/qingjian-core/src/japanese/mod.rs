//! 日语罗马音输入（先只到假名）：罗马音 → 平假名 / 片假名，汉字转换以后再加。

mod composer;
mod romaji;
mod words;

pub use composer::RomajiComposer;
pub use romaji::{to_hiragana, to_katakana};

#[cfg(test)]
mod tests {
    use super::*;

    fn kana(raw: &str) -> String {
        let (kana, pending) = to_hiragana(raw);
        kana + &pending
    }

    #[test]
    fn converts_common_words() {
        assert_eq!(kana("konnnichiha"), "こんにちは");
        assert_eq!(kana("arigatou"), "ありがとう");
        assert_eq!(kana("kitte"), "きって");
        assert_eq!(kana("shinbunn"), "しんぶん");
        assert_eq!(kana("kyouto"), "きょうと");
        assert_eq!(kana("ra-men"), "らーめn");
        assert_eq!(kana("tsukue"), "つくえ");
    }

    #[test]
    fn keeps_unfinished_romaji_pending() {
        assert_eq!(to_hiragana("ky"), (String::new(), "ky".to_owned()));
        assert_eq!(to_hiragana("kan"), ("か".to_owned(), "n".to_owned()));
    }

    #[test]
    fn composer_offers_hiragana_katakana_and_raw() {
        let mut composer = RomajiComposer::default();
        for c in "ramen".chars() {
            composer.push(c);
        }
        assert_eq!(composer.preedit(), "らめn");
        assert_eq!(composer.candidates(), ["らめん", "ラメン", "ramen"]);
        composer.clear();
        for c in "nihon".chars() {
            composer.push(c);
        }
        assert_eq!(composer.candidates()[..3], ["日本", "にほん", "ニホン"]);
        assert!(composer.candidates().contains(&"日本語".to_owned()));
        composer.clear();
        for c in "ko-hi-".chars() {
            composer.push(c);
        }
        assert_eq!(composer.candidates()[0], "コーヒー");
        composer.clear();
        assert!(composer.candidates().is_empty());
    }
}
