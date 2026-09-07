//! 把一段汉字文本按语言模型切成词序列（没有拼音的场合，如接受了云端的整句补全之后要记个人 n-gram）。
//!
//! 与拼音驱动的整句转换不同，这里的词图节点是文本子串：每个位置试 1 到 [`MAX_TEXT_WORD_CHARS`] 个字，
//! 模型认识的子串才算词，单字总可以切出来（模型不认识就扣分）。Viterbi 状态是「切到哪、上一个词是什么」，
//! 打分只用静态模型（个人 bigram 还没记这句，用它反而会自我强化）。标点等非汉字处断句，各句独立。

use super::LanguageModel;

/// 文本切词时一个词最多几个字。
pub const MAX_TEXT_WORD_CHARS: usize = 6;

/// 模型不认识的单字的分数：相当于亿级语料里只出现一次的词，任何认识的词都比它强。
const UNKNOWN_CHAR_LOG_PROB: f64 = -18.5;

/// 切成一句句（非汉字处断开）的词序列。模型一个词都不认识时返回 `None`：没有语言模型（或全是生僻字）时切出来的
/// 只是单字流，记进个人 n-gram 没有意义。
pub fn segment_text(text: &str, model: &dyn LanguageModel) -> Option<Vec<Vec<String>>> {
    let mut clauses = Vec::new();
    let mut known = false;
    for clause in text.split(|c: char| !is_han(c)) {
        let chars: Vec<char> = clause.chars().collect();
        if chars.is_empty() {
            continue;
        }
        let (words, any_known) = segment_clause(&chars, model);
        known |= any_known;
        clauses.push(words);
    }
    (known && !clauses.is_empty()).then_some(clauses)
}

/// 一句里的 Viterbi：节点 = (起点, 词)，每个节点记最优前驱；同一起点、同一个词只留一个节点，
/// 前驱按「上一个词」区分打分。返回词序列和「有没有模型认识的词」。
fn segment_clause(chars: &[char], model: &dyn LanguageModel) -> (Vec<String>, bool) {
    struct Node {
        word: String,
        score: f64,
        known: bool,
        previous: Option<usize>,
    }
    let n = chars.len();
    let mut nodes: Vec<Node> = Vec::new();
    // 每个结束位置上的节点下标
    let mut ending: Vec<Vec<usize>> = vec![Vec::new(); n + 1];
    for start in 0..n {
        let predecessors: Vec<Option<usize>> = if start == 0 {
            vec![None]
        } else {
            ending[start].iter().map(|i| Some(*i)).collect()
        };
        if predecessors.is_empty() {
            continue;
        }
        for len in 1..=MAX_TEXT_WORD_CHARS.min(n - start) {
            let word: String = chars[start..start + len].iter().collect();
            let mut best: Option<Node> = None;
            for predecessor in &predecessors {
                let (previous, base) = match predecessor {
                    Some(i) => (Some(nodes[*i].word.as_str()), nodes[*i].score),
                    None => (None, 0.0),
                };
                let (log_prob, known) = match model.log_prob(previous, &word) {
                    Some(p) => (p, true),
                    None if len == 1 => (UNKNOWN_CHAR_LOG_PROB, false),
                    None => continue,
                };
                let score = base + log_prob;
                if best.as_ref().is_none_or(|b| score > b.score) {
                    best = Some(Node {
                        word: word.clone(),
                        score,
                        known,
                        previous: *predecessor,
                    });
                }
            }
            if let Some(node) = best {
                nodes.push(node);
                ending[start + len].push(nodes.len() - 1);
            }
        }
    }
    let Some(&last) = ending[n]
        .iter()
        .max_by(|a, b| nodes[**a].score.total_cmp(&nodes[**b].score))
    else {
        return (Vec::new(), false);
    };
    let mut path = Vec::new();
    let mut known = false;
    let mut cursor = Some(last);
    while let Some(i) = cursor {
        let node = &nodes[i];
        known |= node.known;
        path.push(node.word.clone());
        cursor = node.previous;
    }
    path.reverse();
    (path, known)
}

/// 是不是汉字（CJK 统一表意文字各区）。
pub(crate) fn is_han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
}

#[cfg(test)]
mod tests {
    use super::super::NoLanguageModel;
    use super::*;

    struct TinyModel;

    impl LanguageModel for TinyModel {
        fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
            let unigram = match word {
                "开发" => -5.0,
                "输入法" => -6.0,
                "输入" => -5.5,
                "法" => -7.0,
                "开" | "发" | "输" | "入" => -8.0,
                "我们" => -4.0,
                "我" => -4.5,
                "们" => -9.0,
                _ => return None,
            };
            let bonus = match (previous, word) {
                (Some("开发"), "输入法") => 2.0,
                _ => 0.0,
            };
            Some(unigram + bonus)
        }
    }

    #[test]
    fn picks_the_best_scoring_word_path() {
        let clauses = segment_text("开发输入法", &TinyModel).unwrap();
        assert_eq!(clauses, [vec!["开发", "输入法"]]);
    }

    #[test]
    fn punctuation_splits_clauses_and_unknown_chars_stay_single() {
        let clauses = segment_text("我们开发，输入法X。", &TinyModel).unwrap();
        assert_eq!(clauses, [vec!["我们", "开发"], vec!["输入法"]]);
        let clauses = segment_text("我们龘", &TinyModel).unwrap();
        assert_eq!(clauses, [vec!["我们", "龘"]]);
    }

    #[test]
    fn without_a_model_nothing_is_learned() {
        assert_eq!(segment_text("开发输入法", &NoLanguageModel), None);
        assert_eq!(segment_text("。。。", &TinyModel), None);
        assert_eq!(segment_text("", &TinyModel), None);
    }
}
