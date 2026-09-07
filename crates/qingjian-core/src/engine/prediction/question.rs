//! 问字模式答案的过滤：模型有时不答字，而是把拼音问题整句「翻译」回来（`?san'ge'zhi'shi'shen'me'zi` → 三个直是什么字）。

/// `answer` 是不是在复述问题而不是作答：与本地转出的问题一样、包含问题的大半，或者问题不短而答案跟它一样长。
/// `guess` 为空（本地转不出问题）时只按「答案里带疑问词」判断。
pub fn restates_question(answer: &str, guess: &str) -> bool {
    let answer = answer.trim();
    if answer.is_empty() {
        return true;
    }
    if answer == guess {
        return true;
    }
    let answer_len = answer.chars().count();
    let guess_len = guess.chars().count();
    if guess_len >= 4 && answer_len >= guess_len {
        return true;
    }
    // 问题的后半截（… 是什么字 / 怎么读 / 什么意思）出现在答案里
    if guess_len >= 4 {
        let tail: String = guess.chars().skip(guess_len / 2).collect();
        if answer.contains(&tail) {
            return true;
        }
    }
    ["什么", "怎么", "哪个", "是不是", "吗"]
        .iter()
        .any(|word| answer_len >= 3 && answer.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restated_questions_are_dropped_and_real_answers_kept() {
        let guess = "三个直是什么字";
        assert!(restates_question("三个直是什么字", guess));
        assert!(restates_question("三个直是什么字？", guess));
        assert!(restates_question("三个直叠在一起是什么字", guess));
        assert!(restates_question("", guess));
        assert!(!restates_question("矗", guess));
        assert!(!restates_question("矗 chù", guess));
    }

    #[test]
    fn short_questions_keep_short_answers() {
        assert!(!restates_question("水", ""));
        assert!(!restates_question("谁", "水"));
        assert!(!restates_question("巴黎", "法国首都"));
        assert!(!restates_question("Paris", ""));
        assert!(restates_question("法国首都是什么", ""));
    }
}
