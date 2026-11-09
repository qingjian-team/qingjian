use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use qingjian_dictionary::Dictionary;

use super::*;

/// 假选词模型：每条都盖住整段，偏爱某个文本。
struct Prefers(&'static str);

impl ChoiceModel for Prefers {
    fn align(&self, _keys: &str, _syllables: &[String]) -> Option<Vec<u32>> {
        Some(vec![1])
    }

    fn letter_units(&self, _keys: &str) -> Vec<u32> {
        vec![1]
    }

    fn score(&self, _context: &str, _typed: &[u32], texts: &[&str]) -> Vec<f32> {
        texts
            .iter()
            .map(|t| if *t == self.0 { 100.0 } else { 0.0 })
            .collect()
    }
}

fn first_texts(engine: &Engine) -> Vec<String> {
    let query = engine.query().unwrap();
    query
        .candidates
        .items
        .iter()
        .take(2)
        .map(|c| c.text.clone())
        .collect()
}

#[test]
fn choice_waits_for_the_shell_then_moves_the_pick_up() {
    let mut engine =
        Engine::new(Dictionary::parse("开发\tkai fa\t9000\n开罚\tkai fa\t10\n").unwrap());
    engine.set_choice_model(Some(Arc::new(Prefers("开罚"))));
    for c in "kaifa".chars() {
        engine.push(c);
    }
    assert_eq!(first_texts(&engine), ["开发", "开罚"]);
    assert!(engine.rescoring_pending());
    // 先出字（这个模型出不了，记下不再算），重查后再给新的一页打分
    settle(&mut engine);
    assert_eq!(first_texts(&engine), ["开罚", "开发"]);
    assert!(!engine.rescoring_pending());
    let note = engine.last_choice.get().expect("排过这页");
    assert_eq!((note.from, note.ctx, note.app), (1, 0, false));
}

/// 假选词模型：选词打分一律相同（不重排），按音节出字时偏爱给定的字。
struct Composes(&'static [char]);

impl ChoiceModel for Composes {
    fn align(&self, keys: &str, syllables: &[String]) -> Option<Vec<u32>> {
        (syllables.concat() == keys).then(|| vec![1; syllables.len()])
    }

    fn letter_units(&self, _keys: &str) -> Vec<u32> {
        vec![1]
    }

    fn score(&self, _context: &str, _typed: &[u32], texts: &[&str]) -> Vec<f32> {
        vec![0.0; texts.len()]
    }

    fn compose(&self, _context: &str, syllables: &[(&str, bool)]) -> Option<CharTable> {
        let index: HashMap<char, u32> = "在再见剑"
            .chars()
            .enumerate()
            .map(|(i, c)| (c, i as u32))
            .collect();
        let rows = (0..syllables.len())
            .map(|_| {
                "在再见剑"
                    .chars()
                    .map(|c| if self.0.contains(&c) { 0.0 } else { -10.0 })
                    .collect()
            })
            .collect();
        Some(CharTable::new(Arc::new(index), rows))
    }
}

fn settle(engine: &mut Engine) {
    let started = Instant::now();
    while engine.rescoring_pending() {
        assert!(engine.request_rescoring());
        while !engine.poll_rescoring() {
            assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
            std::thread::sleep(Duration::from_millis(5));
        }
        engine.query().unwrap();
    }
}

#[test]
fn sentence_uses_the_char_table_once_it_arrives() {
    let mut engine = Engine::new(
        Dictionary::parse("在\tzai\t9000\n再\tzai\t100\n见\tjian\t9000\n剑\tjian\t50\n").unwrap(),
    );
    engine.set_choice_model(Some(Arc::new(Composes(&['再']))));
    for c in "zaijian".chars() {
        engine.push(c);
    }
    // 按键时字分布还没算：照词频；算好后 再 压过词频高的 在
    assert_eq!(first_texts(&engine)[0], "在见");
    settle(&mut engine);
    assert_eq!(first_texts(&engine)[0], "再见");
}

#[test]
fn typing_key_by_key_only_keeps_the_latest_compose_requests() {
    let mut engine = Engine::new(
        Dictionary::parse(
            "在	zai	9000
再	zai	100
见	jian	9000
剑	jian	50
",
        )
        .unwrap(),
    );
    engine.set_choice_model(Some(Arc::new(Composes(&['再']))));
    for c in "zaijianzaijian".chars() {
        engine.push(c);
        engine.query().unwrap();
    }
    let choice = engine.choice.as_ref().unwrap();
    let wanted = choice.compose_wanted.borrow();
    assert!(!wanted.is_empty());
    assert!(
        wanted.iter().all(|request| request.syllables.len() == 4),
        "只该剩整段输入的请求：{:?}",
        wanted.iter().map(|r| &r.syllables).collect::<Vec<_>>()
    );
}

#[test]
fn altered_readings_get_a_fixed_cost_not_the_table() {
    let index: HashMap<char, u32> = [('再', 0), ('在', 1)].into_iter().collect();
    let table = CharTable::new(Arc::new(index), vec![vec![0.0, -6.0]]);
    let typed = [qingjian_dictionary::SyllablePattern::complete("zai")];
    let reading = |s: &str| vec![s.to_owned()];
    assert_eq!(char_emission(&table, &typed, 0, "再", &reading("zai")), 0.0);
    assert_eq!(
        char_emission(&table, &typed, 0, "在", &reading("zai")),
        -6.0 * EMISSION_WEIGHT
    );
    // 敲错边读成 cai 的字：字分布是按 zai 出的，说明不了它
    assert_eq!(
        char_emission(&table, &typed, 0, "才", &reading("cai")),
        ALTERED_EMISSION * EMISSION_WEIGHT
    );
}

#[test]
fn the_model_picks_from_more_sentences_than_the_list_shows() {
    let mut engine =
        Engine::new(Dictionary::parse("是\tshi\t1000\n时\tshi\t100\n事\tshi\t10\n").unwrap());
    // 两处不是最常用字：词图里排在第九，显示的只有前三条
    engine.set_choice_model(Some(Arc::new(Prefers("是是时时"))));
    // 准确档看前 18 条：16 条整句都在模型的窗口里
    engine.set_choice_tier(ChoiceTier::Accurate);
    for c in "shishishishi".chars() {
        engine.push(c);
    }
    let sentences = |engine: &Engine| {
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .filter(|c| c.kind == CandidateKind::Sentence)
            .map(|c| c.text.clone())
            .collect::<Vec<_>>()
    };
    let before = sentences(&engine);
    assert_eq!(before.len(), crate::engine::SENTENCE_CANDIDATES);
    assert!(!before.contains(&"是是时时".to_owned()), "{before:?}");
    settle(&mut engine);
    let after = sentences(&engine);
    assert_eq!(after.len(), crate::engine::SENTENCE_CANDIDATES);
    assert_eq!(after[0], "是是时时", "{after:?}");
    assert_eq!(first_texts(&engine)[0], "是是时时");
}

#[test]
fn the_shell_is_woken_once_the_result_is_ready() {
    let mut engine =
        Engine::new(Dictionary::parse("开发\tkai fa\t9000\n开罚\tkai fa\t10\n").unwrap());
    // 模型后接上、回调先给，或者反过来，都要叫得到
    let (sender, woken) = std::sync::mpsc::channel();
    engine.set_rescoring_wake(Some(Arc::new(move || {
        let _ = sender.send(());
    })));
    engine.set_choice_model(Some(Arc::new(Prefers("开罚"))));
    for c in "kaifa".chars() {
        engine.push(c);
    }
    engine.query().unwrap();
    assert!(engine.request_rescoring());
    woken
        .recv_timeout(Duration::from_secs(5))
        .expect("结果到了要叫醒壳");
    assert!(engine.poll_rescoring(), "叫醒时结果已经在通道里");
}

/// 假选词模型：像知意一样对齐，音节可以只敲了开头（`su` 对 `sui`，简拼就是这么对上的），偏爱某个文本。
struct PrefixPrefers(&'static str);

impl ChoiceModel for PrefixPrefers {
    fn align(&self, keys: &str, syllables: &[String]) -> Option<Vec<u32>> {
        let mut rest = keys;
        for syllable in syllables {
            let common = rest
                .bytes()
                .zip(syllable.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            if common == 0 {
                return None;
            }
            rest = &rest[common..];
        }
        rest.is_empty().then(|| vec![1; syllables.len()])
    }

    fn letter_units(&self, _keys: &str) -> Vec<u32> {
        vec![1]
    }

    fn score(&self, _context: &str, _typed: &[u32], texts: &[&str]) -> Vec<f32> {
        texts
            .iter()
            .map(|t| if *t == self.0 { 100.0 } else { 0.0 })
            .collect()
    }
}

/// 只认几个词的语言模型：敲错边（`su` 少敲一个 i）读出的 虽然 分数压过原样的 肃然。
struct SuranModel;

impl crate::sentence::LanguageModel for SuranModel {
    fn log_prob(&self, _previous: Option<&str>, word: &str) -> Option<f64> {
        match word {
            "虽然" => Some(-5.0),
            "肃然" => Some(-14.0),
            "然" => Some(-9.0),
            "苏" => Some(-9.0),
            _ => None,
        }
    }
}

#[test]
fn the_choice_model_does_not_promote_a_corrected_sentence() {
    // 原样读得通（肃然），敲错边读出的 虽然 只当第二条。知意会把它当成 `su` 是 `sui` 的简拼对齐上，
    // 再偏爱它也不许提到第一：重排里的语言模型分与位置先验只看词常不常用（上屏 → 商品、这俩 → 这里）
    let dictionary = "肃然	su ran	500
虽然	sui ran	900000
苏	su	5000
然	ran	9000
";
    let mut engine = Engine::new(Dictionary::parse(dictionary).unwrap())
        .with_language_model(Box::new(SuranModel));
    engine.set_choice_model(Some(Arc::new(PrefixPrefers("虽然"))));
    engine.set_input("suran");
    let before: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert_eq!(before[0], "肃然", "{before:?}");
    assert!(before.iter().any(|t| t == "虽然"), "{before:?}");
    settle(&mut engine);
    assert_eq!(first_texts(&engine)[0], "肃然");
}
