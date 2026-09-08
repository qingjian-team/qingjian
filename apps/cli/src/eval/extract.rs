//! 从用户的中文文本里抽评测句：一行是一个段落，段落按非汉字（标点、英文、数字、空格）切成小句，
//! 只留纯汉字、长度在范围内的小句，每句带上它在段落里前面的真实文本当上文。

use super::pair::MAX_CONTEXT_CHARS;

/// 一句至少几个字：两个字大多是一个词，评不出整句。
pub const MIN_SENTENCE_CHARS: usize = 3;

/// 一句最多几个字：再长用户也会中途上屏，评它没有意义。
pub const MAX_SENTENCE_CHARS: usize = 20;

/// 抽出来的一句：汉字与它前面的上文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    pub text: String,
    pub context: String,
}

/// 按段落抽句；同一段落里上文按原文累积（含标点），截到最后 [`MAX_CONTEXT_CHARS`] 个字符。
pub fn extract(text: &str) -> Vec<Extracted> {
    let mut out = Vec::new();
    for paragraph in text.lines() {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        let mut context = String::new();
        let mut clause = String::new();
        let mut flush = |clause: &mut String, context: &mut String| {
            let count = clause.chars().count();
            if (MIN_SENTENCE_CHARS..=MAX_SENTENCE_CHARS).contains(&count) {
                out.push(Extracted {
                    text: clause.clone(),
                    context: last_chars(context, MAX_CONTEXT_CHARS),
                });
            }
            context.push_str(clause);
            clause.clear();
        };
        for c in paragraph.chars() {
            if is_han(c) {
                clause.push(c);
            } else {
                flush(&mut clause, &mut context);
                context.push(c);
            }
        }
        flush(&mut clause, &mut context);
    }
    out
}

fn last_chars(text: &str, count: usize) -> String {
    let total = text.chars().count();
    text.chars().skip(total.saturating_sub(count)).collect()
}

/// 与 Core `sentence::text_segment::is_han` 同一套范围。
fn is_han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_paragraphs_into_han_clauses_with_context() {
        let text = "我今天想去上海，然后 call 一下老王。\n\n第二段太短了。\n";
        let got = extract(text);
        let texts: Vec<&str> = got.iter().map(|e| e.text.as_str()).collect();
        // 两个字的「然后」不够长，不评
        assert_eq!(texts, ["我今天想去上海", "一下老王", "第二段太短了"]);
        assert_eq!(got[0].context, "");
        assert_eq!(got[1].context, "我今天想去上海，然后 call ");
        assert_eq!(got[2].context, "");
    }

    #[test]
    fn drops_clauses_out_of_range() {
        let long = "字".repeat(MAX_SENTENCE_CHARS + 1);
        let text = format!("好。可以。{long}。");
        let got = extract(&text);
        assert!(got.is_empty());
    }
}
