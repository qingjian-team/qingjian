//! 成对符号（#928）：中文引号的左右交替与补右半边、跳过配合得上；英文缺省不成对；离开光标后不再认补过的右半边。

use super::*;
use crate::PairEdit;

#[test]
fn chinese_quotes_pair_and_the_closing_quote_skips() {
    let mut engine = engine();
    let open = engine.punctuate('"').unwrap();
    assert_eq!(
        engine.pair_punctuation(open, false),
        Some(PairEdit::Insert {
            text: "“”".to_owned(),
            back: 1
        })
    );
    // 中间打了字，再敲 " 转出右引号，正是补上的那个
    engine.note_passthrough('a');
    let close = engine.punctuate('"').unwrap();
    assert_eq!(close, "”");
    assert_eq!(
        engine.pair_punctuation(close, false),
        Some(PairEdit::Skip { closer: '”' })
    );
}

#[test]
fn a_stale_quote_toggle_still_opens_a_pair() {
    // 交替状态停在「下一个是右引号」（上次只敲了左引号）：开着成对时仍补一对左右引号
    let mut engine = engine();
    engine.set_pair_punctuation(false, false);
    assert_eq!(engine.punctuate('"'), Some("“"));
    engine.set_pair_punctuation(true, false);
    let quote = engine.punctuate('"').unwrap();
    assert_eq!(quote, "”");
    assert_eq!(
        engine.pair_punctuation(quote, false),
        Some(PairEdit::Insert {
            text: "“”".to_owned(),
            back: 1
        })
    );
    // 接着敲 " 转出右引号，跳过补上的那个；再敲又是左引号
    let close = engine.punctuate('"').unwrap();
    assert_eq!(
        engine.pair_punctuation(close, false),
        Some(PairEdit::Skip { closer: '”' })
    );
    assert_eq!(engine.punctuate('"'), Some("“"));
}

#[test]
fn english_pairs_only_when_switched_on() {
    let mut engine = engine();
    assert_eq!(engine.pair_punctuation("(", true), None);
    engine.set_pair_punctuation(true, true);
    assert_eq!(
        engine.pair_punctuation("(", true),
        Some(PairEdit::Insert {
            text: "()".to_owned(),
            back: 1
        })
    );
    assert_eq!(engine.pair_backspace(), Some(('(', ')')));
}

#[test]
fn leaving_the_caret_forgets_inserted_closers() {
    let mut engine = engine();
    let open = engine.punctuate('(').unwrap();
    assert!(engine.pair_punctuation(open, false).is_some());
    engine.break_chain();
    assert_eq!(engine.pair_punctuation("）", false), None);
    assert_eq!(engine.pair_backspace(), None);
}
