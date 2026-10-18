//! 生词本：上屏带译词的词，那句话写完时交出整句当例句；每次上屏有学习语言说法的中文记一次相遇。

use super::*;

type Examples = Arc<Mutex<Vec<(String, String, String)>>>;

type Encounters = Arc<Mutex<Vec<Encounter>>>;

/// 只收例句与相遇的词汇记录。
#[derive(Default)]
struct ExampleBook(Examples, Encounters);

impl VocabularyTracker for ExampleBook {
    fn exposures(&self, _language: Language, _word: &str) -> u32 {
        0
    }

    fn record_exposure(&mut self, _language: Language, _word: &str) {}

    fn record_commit(&mut self, _language: Language, _word: &str, _used: bool) {}

    fn record_encounter(&mut self, encounter: &Encounter) {
        self.1.lock().unwrap().push(encounter.clone());
    }

    fn record_example(&mut self, _language: Language, word: &str, source: &str, example: &str) {
        self.0
            .lock()
            .unwrap()
            .push((word.to_owned(), source.to_owned(), example.to_owned()));
    }
}

fn pick(engine: &mut Engine, input: &str, text: &str) {
    engine.set_input(input);
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let candidate = query
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == text)
        .unwrap();
    engine.commit(&candidate);
}

#[test]
fn a_translated_word_gets_the_sentence_it_was_typed_in() {
    let examples = Examples::default();
    let mut engine = engine()
        .with_translator(Box::new(FixedTranslator))
        .with_vocabulary_tracker(Box::new(ExampleBook(
            examples.clone(),
            Encounters::default(),
        )));
    pick(&mut engine, "xian", "先");
    pick(&mut engine, "kaifa", "开发");
    assert!(examples.lock().unwrap().is_empty(), "句子还没写完");
    assert_eq!(engine.punctuate('.'), Some("。"));
    assert_eq!(
        *examples.lock().unwrap(),
        [(
            "develop".to_owned(),
            "开发".to_owned(),
            "先开发。".to_owned()
        )]
    );

    // 换了应用：没打句号也算这句写完
    pick(&mut engine, "kaifa", "开发");
    pick(&mut engine, "xia", "下");
    engine.break_chain();
    assert_eq!(examples.lock().unwrap()[1].2, "开发下");
}

#[test]
fn private_input_leaves_no_example() {
    let examples = Examples::default();
    let mut engine = engine()
        .with_translator(Box::new(FixedTranslator))
        .with_vocabulary_tracker(Box::new(ExampleBook(
            examples.clone(),
            Encounters::default(),
        )));
    pick(&mut engine, "kaifa", "开发");
    // 词还在等句子写完时进了密码框：等着的那句扔掉，密码框里打的也不接上
    engine.set_private(true);
    pick(&mut engine, "kaifa", "开发");
    engine.punctuate('.');
    engine.set_private(false);
    pick(&mut engine, "xian", "先");
    engine.punctuate('.');
    assert!(examples.lock().unwrap().is_empty());
}

/// 记下要翻的整段，翻译结果由测试塞进去。
#[derive(Default)]
struct PhraseFiller {
    asked: Arc<Mutex<Vec<String>>>,

    done: Arc<Mutex<Vec<Encounter>>>,
}

impl GlossFiller for PhraseFiller {
    fn request(&mut self, _language: Language, _word: &str) {}

    fn poll(&mut self) -> Vec<FilledGloss> {
        Vec::new()
    }

    fn translate(&mut self, _language: Language, text: &str) {
        self.asked.lock().unwrap().push(text.to_owned());
    }

    fn poll_translations(&mut self) -> Vec<Encounter> {
        std::mem::take(&mut *self.done.lock().unwrap())
    }
}

#[test]
fn commits_with_a_translation_become_encounters_and_sentences_go_to_the_cloud() {
    let encounters = Encounters::default();
    let filler = PhraseFiller::default();
    let (asked, done) = (filler.asked.clone(), filler.done.clone());
    let mut engine = engine()
        .with_translator(Box::new(FixedTranslator))
        .with_gloss_filler(Box::new(filler))
        .with_vocabulary_tracker(Box::new(ExampleBook(
            Examples::default(),
            encounters.clone(),
        )));
    pick(&mut engine, "kaifa", "开发");
    // 单字不记
    pick(&mut engine, "xian", "先");
    assert_eq!(
        *encounters.lock().unwrap(),
        [Encounter {
            language: Language::English,
            text: "开发".to_owned(),
            translation: "develop".to_owned(),
            reading: None,
        }]
    );

    // 整句没有现成译词：交给云端翻，翻好了进相遇
    let sentence = Candidate {
        text: "先开发".to_owned(),
        kind: CandidateKind::Sentence,
        syllables: vec!["xian".to_owned(), "kai".to_owned(), "fa".to_owned()],
        reading: None,
        translation: None,
        aux_code: None,
    };
    engine.set_input("xiankaifa");
    engine.commit(&sentence);
    assert_eq!(*asked.lock().unwrap(), ["先开发"]);
    done.lock().unwrap().push(Encounter {
        language: Language::English,
        text: "先开发".to_owned(),
        translation: "develop first".to_owned(),
        reading: None,
    });
    engine.poll_glosses();
    assert_eq!(encounters.lock().unwrap()[1].translation, "develop first");

    // 私密输入中不记也不发
    engine.set_private(true);
    pick(&mut engine, "kaifa", "开发");
    engine.set_input("xiankaifa");
    engine.commit(&sentence);
    assert_eq!(encounters.lock().unwrap().len(), 2);
    assert_eq!(asked.lock().unwrap().len(), 1);
}
