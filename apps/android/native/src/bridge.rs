//! Android 原生门面：沿用核心查询、译词与学习，所有调用来自同一工作线程。

use std::path::Path;

use qingjian_core::{Engine, Language};
use qingjian_dictionary::Dictionary;
use qingjian_learning::FrequencyLearner;
use qingjian_lm::BigramModel;
use qingjian_translate::Glossary;
use serde_json::{Value, json};

pub struct Bridge {
    engine: Engine,

    /// 用无学习的引擎测量核心实际消耗的拼音，不在手机壳里复制音节对齐。
    probe: Engine,
}

impl Bridge {
    pub fn open(data: &Path, user: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(user).map_err(|e| e.to_string())?;
        let dictionary = Dictionary::from_path(data.join("dict.qj")).map_err(|e| e.to_string())?;
        let probe_dictionary =
            Dictionary::from_path(data.join("dict.qj")).map_err(|e| e.to_string())?;
        let glossary = Glossary::from_path(Language::English, data.join("glossary-en.qj"))
            .map_err(|e| e.to_string())?;
        let learner =
            FrequencyLearner::from_path(user.join("frequency.tsv")).map_err(|e| e.to_string())?;
        let mut engine = Engine::new(dictionary)
            .with_translator(Box::new(glossary))
            .with_learner(Box::new(learner));
        let lm = data.join("lm.qj");
        if lm.is_file() {
            let model = BigramModel::from_path(&lm).map_err(|e| e.to_string())?;
            engine = engine.with_language_model(Box::new(model));
        }
        let mut probe = Engine::new(probe_dictionary);
        probe.set_learning(false);
        Ok(Self { engine, probe })
    }

    pub fn query(&mut self, input: &str) -> Value {
        self.query_frame(input, false)
    }

    pub fn annotate(&mut self, input: &str) -> Value {
        if self.engine.composition().text() != input {
            return json!({"candidates": [], "error": null});
        }
        self.query_frame(input, true)
    }

    fn query_frame(&mut self, input: &str, annotate: bool) -> Value {
        if input.len() > 96 || !input.bytes().all(|b| b.is_ascii_lowercase() || b == b'\'') {
            return json!({"candidates": [], "error": "无效或过长的拼音"});
        }
        if !annotate {
            self.engine.clear();
            self.engine.set_input(input);
        }
        let mut result = match self.engine.query() {
            Ok(result) => result,
            Err(_) => return json!({"candidates": [], "error": null}),
        };
        result.candidates.items.truncate(32);
        if annotate {
            self.engine.annotate(&mut result.candidates);
        }
        let candidates: Vec<Value> = result
            .candidates
            .items
            .iter()
            .filter_map(|candidate| {
                self.probe.clear();
                self.probe.set_input(input);
                self.probe.commit(candidate);
                let consumed = input
                    .len()
                    .saturating_sub(self.probe.composition().text().len());
                if consumed == 0 {
                    return None;
                }
                let gloss = candidate
                    .translation
                    .as_ref()
                    .map(|t| {
                        t.senses()
                            .iter()
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" · ")
                    })
                    .unwrap_or_default();
                Some(json!({"text": candidate.text, "gloss": gloss, "consumed": consumed}))
            })
            .collect();
        json!({"candidates": candidates, "error": null})
    }

    pub fn learn(&mut self, text: &str, input: &str) {
        if input.len() > 96 || !input.bytes().all(|b| b.is_ascii_lowercase() || b == b'\'') {
            return;
        }
        self.engine.clear();
        self.engine.set_input(input);
        if let Ok(result) = self.engine.query()
            && let Some(candidate) = result.candidates.items.iter().find(|c| c.text == text)
        {
            self.engine.commit(candidate);
            self.engine.flush_learning();
        }
        self.engine.clear();
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.engine.flush_learning();
    }
}
