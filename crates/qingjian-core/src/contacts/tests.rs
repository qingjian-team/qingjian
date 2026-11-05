//! 名字的读音与通讯录词库的收录规则。

use super::reading::readings;
use super::{Contact, ContactBook};

fn read(name: &str, words: &[(&str, &str)]) -> Vec<String> {
    readings(
        name,
        |syllables| {
            words
                .iter()
                .any(|(text, pinyin)| *text == name && syllables.join(" ") == *pinyin)
        },
        |_| None,
    )
}

fn contact(name: &str, details: &[&str]) -> Contact {
    Contact {
        name: name.to_owned(),
        details: details.iter().map(|d| (*d).to_owned()).collect(),
    }
}

#[test]
fn polyphonic_surnames_read_as_surnames() {
    assert_eq!(read("曾小明", &[]), ["zeng xiao ming"]);
    assert_eq!(read("单田芳", &[]), ["shan tian fang"]);
    assert_eq!(read("尉迟恭", &[]), ["yu chi gong"]);
    // 不是姓的开头照常逐字
    assert_eq!(read("张伟", &[]), ["zhang wei"]);
}

#[test]
fn dictionary_words_keep_their_reading() {
    // 长 最常见读 chang，词库里 长辈 读 zhang bei
    assert_eq!(read("长辈", &[("长辈", "zhang bei")]), ["zhang bei"]);
    // 按姓读与词库读不一样时两条都收，姓在前
    assert_eq!(
        read("曾经", &[("曾经", "ceng jing")]),
        ["zeng jing", "ceng jing"]
    );
}

#[test]
fn unreadable_names_are_skipped() {
    assert!(readings("张\u{2A6D6}", |_| false, |_| None).is_empty());
}

#[test]
fn only_short_han_names_are_kept_and_details_merge() {
    let book = ContactBook::build(
        &[
            contact("张 伟", &["138 0013 8000"]),
            contact("张伟", &["zw@example.com", "138 0013 8000"]),
            contact("Tom", &["123"]),
            contact("妈", &["456"]),
            contact("王师傅修空调电话", &["789"]),
        ],
        |_, _| false,
        |_| None,
    );
    assert_eq!(book.len(), 1);
    assert!(book.contains("张伟"));
    assert!(!book.contains("Tom") && !book.contains("妈"));
    assert_eq!(book.details("张伟"), ["138 0013 8000", "zw@example.com"]);
    assert!(book.mentioned_in("我和张伟去"));
    assert!(!book.mentioned_in("张三"));
}
