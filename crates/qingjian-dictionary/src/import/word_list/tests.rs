use super::*;

fn reference(entries: &[(&str, &str, u32)]) -> Dictionary {
    let source: String = entries
        .iter()
        .map(|(text, pinyin, frequency)| format!("{text}\t{pinyin}\t{frequency}\n"))
        .collect();
    Dictionary::parse(&source).unwrap()
}

fn texts(derived: &Derived) -> Vec<String> {
    derived
        .tsv
        .lines()
        .map(|line| line.split('\t').nth(1).unwrap().to_owned())
        .collect()
}

#[test]
fn reads_plain_word_lists_weights_and_readings() {
    let words =
        parse_words("# 名字\n张三\n李四\t500\n王五\t\n美露莘\tmei lu xin\n赵六\tmei lu xin\t3\n");
    assert_eq!(
        words,
        [
            Word {
                text: "张三".to_owned(),
                reading: None,
                weight: 1
            },
            Word {
                text: "李四".to_owned(),
                reading: None,
                weight: 500
            },
            Word {
                text: "王五".to_owned(),
                reading: None,
                weight: 1
            },
            Word {
                text: "美露莘".to_owned(),
                reading: Some("mei lu xin".to_owned()),
                weight: 1
            },
            Word {
                text: "赵六".to_owned(),
                reading: Some("mei lu xin".to_owned()),
                weight: 3
            },
        ]
    );
    // 同一个词只留第一次
    assert_eq!(parse_words("张三\n张三\t500\n").len(), 1);
}

#[test]
fn skips_header_of_rime_word_lists() {
    let words = parse_words(
        "# Rime dictionary\n---\nname: names\nversion: \"1\"\nsort: by_weight\n...\n张三\n李四\t7\n",
    );
    assert_eq!(words.len(), 2);
    assert_eq!(words[0].text, "张三");
    assert_eq!(words[1].weight, 7);
}

#[test]
fn skips_lines_with_an_unreadable_second_column() {
    // 第二列既不是词频也不是读音（这里带数字），整行跳过
    assert_eq!(parse_words("张三\tzhang san 1\n李四\n").len(), 1);
}

#[test]
fn uses_the_reading_written_in_the_list() {
    // 词表自己写了读音就用它：多音字词只有用户知道怎么读
    let dictionary = reference(&[("美", "mei", 900), ("露", "lu", 900), ("莘", "shen", 900)]);
    let derived = derive(
        &[Word {
            text: "美露莘".to_owned(),
            reading: Some("mei lu xin".to_owned()),
            weight: 1,
        }],
        &[&dictionary],
    );
    assert_eq!(texts(&derived), ["mei lu xin"]);
}

#[test]
fn uses_the_dictionary_reading_of_a_known_word() {
    // 整词在词库里就用词库读音，不派生（建设银行 不会出 yin xing / yin heng）
    let dictionary = reference(&[
        ("建", "jian", 900),
        ("设", "she", 900),
        ("银", "yin", 900),
        ("行", "xing", 900),
        ("行", "hang", 800),
        ("行", "heng", 100),
        ("建设银行", "jian she yin hang", 23),
    ]);
    let derived = derive(
        &[Word {
            text: "建设银行".to_owned(),
            reading: None,
            weight: 1,
        }],
        &[&dictionary],
    );
    assert_eq!(texts(&derived), ["jian she yin hang"]);
    assert_eq!(derived.report.words, 1);
}

#[test]
fn segments_unknown_words_by_dictionary_words() {
    // 中华人民共和国 整体不在词库，靠切分躲开「和」的多音
    let dictionary = reference(&[
        ("中华", "zhong hua", 900),
        ("人民", "ren min", 900),
        ("共和国", "gong he guo", 900),
        ("和", "he", 900),
        ("和", "hu", 100),
    ]);
    let derived = derive(
        &[Word {
            text: "中华人民共和国".to_owned(),
            reading: None,
            weight: 1,
        }],
        &[&dictionary],
    );
    assert_eq!(texts(&derived), ["zhong hua ren min gong he guo"]);
    assert_eq!(derived.report.words, 1);
}

#[test]
fn writes_one_entry_per_word_however_long_it_is() {
    // 字一多也只写一条，不再因为组合数超上限整条丢掉
    let dictionary = reference(&[
        ("长", "chang", 900),
        ("春", "chun", 900),
        ("市", "shi", 900),
        ("朝", "chao", 900),
        ("阳", "yang", 900),
        ("区", "qu", 900),
        ("大街", "da jie", 900),
        ("长春市", "chang chun shi", 503),
    ]);
    let derived = derive(
        &[
            Word {
                text: "长春市朝阳区".to_owned(),
                reading: None,
                weight: 1,
            },
            Word {
                text: "长春市朝阳区大街".to_owned(),
                reading: None,
                weight: 1,
            },
        ],
        &[&dictionary],
    );
    assert_eq!(
        texts(&derived),
        [
            "chang chun shi chao yang qu",
            "chang chun shi chao yang qu da jie"
        ]
    );
    assert!(derived.report.unknown.is_empty());
}

#[test]
fn takes_the_most_frequent_reading_of_a_lone_char() {
    let dictionary = reference(&[("和", "he", 900), ("和", "hu", 90), ("和", "huo", 3)]);
    let derived = derive(
        &[Word {
            text: "和".to_owned(),
            reading: None,
            weight: 1,
        }],
        &[&dictionary],
    );
    assert_eq!(texts(&derived), ["he"]);
}

#[test]
fn skips_words_whose_char_is_not_in_the_reference() {
    let dictionary = reference(&[("三", "san", 900)]);
    let derived = derive(
        &[Word {
            text: "囧三".to_owned(),
            reading: None,
            weight: 1,
        }],
        &[&dictionary],
    );
    assert!(derived.tsv.is_empty());
    assert_eq!(derived.report.unknown, ["囧三".to_owned()]);
    assert_eq!(derived.report.words, 0);
}
