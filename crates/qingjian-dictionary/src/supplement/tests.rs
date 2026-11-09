use super::*;

/// 拟直线要的重合词：第 i 个的词频是 100 + 10i，词表里的 log10 P 与它对应（ln f = log10 P · ln 10 + 6 ln 10）。
fn overlap(i: usize) -> (String, u32) {
    let text = format!("测{}", char::from_u32(0x4e00 + i as u32).unwrap());
    (text, 100 + 10 * i as u32)
}

fn reference() -> Dictionary {
    let mut source = String::from(
        "摸\tmo\t5000\n鱼\tyu\t8000\n老\tlao\t9000\n梗\tgeng\t300\n进行\tjin xing\t9000\n分析\tfen xi\t8000\n\
         行\txing\t9000\n行\thang\t5000\n银行\tyin hang\t6000\n银\tyin\t3000\n",
    );
    for i in 0..MIN_FIT_PAIRS {
        let (text, frequency) = overlap(i);
        source.push_str(&format!("{text}\tce ce\t{frequency}\n"));
    }
    Dictionary::parse(&source).unwrap()
}

fn vocabulary() -> Vec<(String, f64)> {
    let mut words: Vec<(String, f64)> = (0..MIN_FIT_PAIRS)
        .map(|i| {
            let (text, frequency) = overlap(i);
            (text, f64::from(frequency).log10() - 6.0)
        })
        .collect();
    for word in ["摸鱼", "老梗", "进行分析", "行鱼", "<unk>", "鱼a"] {
        words.push((word.to_owned(), -5.0));
    }
    // 词库已有的词也在词表里，概率与词频对得上
    words.push(("分析".to_owned(), 8000f64.log10() - 6.0));
    words
}

fn added() -> Dictionary {
    let reference = reference();
    let words = vocabulary();
    supplement(words.iter().map(|(t, p)| (t.as_str(), *p)), &[&reference]).unwrap()
}

#[test]
fn missing_words_are_added_with_guessed_readings() {
    let added = added();
    assert_eq!(added.lookup(&["mo", "yu"], false)[0].text, "摸鱼");
    assert_eq!(added.lookup(&["lao", "geng"], false)[0].text, "老梗");
    // log10 P = −5 按拟出的直线是 ln f = ln 10
    assert_eq!(added.lookup(&["mo", "yu"], false)[0].frequency, 10);
}

#[test]
fn known_words_fragments_and_non_han_tokens_are_skipped() {
    let texts: HashSet<String> = added().entries().map(|m| m.text.to_owned()).collect();
    // 分析 已有，进行分析 = 进行 + 分析，<unk> 与 鱼a 不是汉字词，重合词本来就有
    assert_eq!(
        texts,
        HashSet::from(["摸鱼", "老梗", "行鱼"].map(String::from))
    );
}

#[test]
fn a_common_minor_reading_gets_its_own_entry() {
    let added = added();
    // 行 的 hang 在 银行 里用得多，占比过线：行鱼 两种读音都收
    assert_eq!(added.lookup(&["xing", "yu"], false)[0].text, "行鱼");
    assert_eq!(added.lookup(&["hang", "yu"], false)[0].text, "行鱼");
}

#[test]
fn too_little_overlap_adds_nothing() {
    let reference = Dictionary::parse("摸\tmo\t5000\n鱼\tyu\t8000\n").unwrap();
    let added = supplement([("摸鱼", -5.0)].into_iter(), &[&reference]).unwrap();
    assert!(added.is_empty());
}
