//! 拼音以外的几种查询：表达式（`v`）、原样字母、英文模式、问字（`?`）。

use super::*;

impl Engine {
    /// 表达式模式（`v` 开头）：不解析拼音，候选是算式结果 / 中文数字，再加上整段是英文词的情况（`very`）。
    /// preedit 原样显示输入。
    pub(in crate::engine) fn query_expression(
        &self,
        scope: &str,
        rest: String,
        start: Instant,
    ) -> Query {
        let mut items = shortcut::candidates(scope, self.modes().expression, &jiff::Zoned::now());
        if let Some(candidate) = self.english_word_candidate(scope) {
            items.push(candidate);
        }
        Query {
            segmentations: Vec::new(),
            spelled: 0,
            candidates: CandidateList { items },
            tail: scope.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: self.shuangpin.is_some() || self.zhuyin,
            shuangpin_raw_preedit: false,
            typed_display: None,
            typed_case: None,
            pinyin_choices: Vec::new(),
            correction: None,
            aux: None,
            held: String::new(),
            timings: Timings {
                parse: Duration::ZERO,
                lookup: Duration::ZERO,
                rank: start.elapsed(),
            },
        }
    }

    /// 英文直输段：首选就是原文（`no-way`），空格 / 回车都上屏它；preedit 原样显示。
    /// 原文是「用户名@」（`qingjian@`、`zhang.san@q`）时后面跟常见邮箱的整个地址，不记学习。
    pub(in crate::engine) fn query_raw(&self, scope: &str, rest: String, start: Instant) -> Query {
        let mut items = vec![Candidate {
            text: scope.to_owned(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        }];
        items.extend(
            crate::email_completions(scope)
                .into_iter()
                .map(|completion| Candidate {
                    text: format!("{scope}{}", completion.insert),
                    kind: CandidateKind::Shortcut,
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                    aux_code: None,
                }),
        );
        Query {
            segmentations: Vec::new(),
            spelled: 0,
            candidates: CandidateList { items },
            tail: scope.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: self.shuangpin.is_some() || self.zhuyin,
            shuangpin_raw_preedit: false,
            typed_display: None,
            typed_case: None,
            pinyin_choices: Vec::new(),
            correction: None,
            aux: None,
            held: String::new(),
            timings: Timings {
                parse: Duration::ZERO,
                lookup: Duration::ZERO,
                rank: start.elapsed(),
            },
        }
    }

    /// 英文模式：敲的字母原样显示，候选是英文词表的精确词、前缀补全与拼错纠正（见 [`english::suggest`]），
    /// 词表没装就没有候选。emoji 照配，但排在所有词后面：选词靠上下键，emoji 夹在词中间会挡路。
    pub(in crate::engine) fn query_english(
        &self,
        scope: &str,
        rest: String,
        start: Instant,
    ) -> Query {
        let mut items: Vec<Candidate> = english::suggest(
            &self.english_lists(),
            scope,
            |text| self.learner.weight(text),
            ENGLISH_MODE_CANDIDATES,
        )
        .into_iter()
        .map(|text| Candidate {
            text,
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        })
        .collect();
        self.insert_emoji(&mut items);
        items.sort_by_key(|c| c.kind.follows_word());
        Query {
            segmentations: Vec::new(),
            spelled: 0,
            candidates: CandidateList { items },
            tail: scope.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: self.shuangpin.is_some() || self.zhuyin,
            shuangpin_raw_preedit: false,
            typed_display: None,
            typed_case: None,
            pinyin_choices: Vec::new(),
            correction: None,
            aux: None,
            held: String::new(),
            timings: Timings {
                parse: Duration::ZERO,
                lookup: Duration::ZERO,
                rank: start.elapsed(),
            },
        }
    }

    /// 问字模式（问字键或 `?` 开头）：十六进制码点（`u4e00`、`u+1f600`）本地直接给出那个字符；
    /// 整段是英文词时（`unchanged`）给英文候选，否则本地没有候选，preedit 显示前缀加切分好的问题拼音，答案等云端。
    pub(in crate::engine) fn query_question(
        &self,
        scope: &str,
        rest: String,
        start: Instant,
    ) -> Query {
        let body = self.modes().question_body(scope, self.zhuyin);
        let prefix = &scope[..scope.len() - body.len()];
        let (candidates, tail) = match shortcut::unicode_form(body) {
            Some(text) => (
                CandidateList {
                    items: vec![Candidate {
                        text,
                        kind: CandidateKind::Shortcut,
                        syllables: Vec::new(),
                        reading: None,
                        translation: None,
                        aux_code: None,
                    }],
                },
                scope.to_owned(),
            ),
            None => match self.english_word_candidate(scope) {
                // u 开头的英文词不当成问字问题：原样给词，空格可直接上屏
                Some(candidate) => (
                    CandidateList {
                        items: vec![candidate],
                    },
                    scope.to_owned(),
                ),
                None => (
                    CandidateList::default(),
                    format!("{prefix}{}", self.marked_rest(body)),
                ),
            },
        };
        Query {
            segmentations: Vec::new(),
            spelled: 0,
            candidates,
            tail,
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: self.shuangpin.is_some() || self.zhuyin,
            shuangpin_raw_preedit: false,
            typed_display: None,
            typed_case: None,
            pinyin_choices: Vec::new(),
            correction: None,
            aux: None,
            held: String::new(),
            timings: Timings {
                parse: start.elapsed(),
                lookup: Duration::ZERO,
                rank: Duration::ZERO,
            },
        }
    }

    /// 整段输入在英文词表（用户词表优先、随包词表在后）精确命中时的候选。
    /// 表达式（`very`）与问字（`unchanged`）两种前缀模式共用：前缀字母不拼拼音，整段是词就让用户直接上屏。
    pub(in crate::engine) fn english_word_candidate(&self, scope: &str) -> Option<Candidate> {
        let code = scope.to_ascii_lowercase();
        let word = self
            .english_lists()
            .iter()
            .find_map(|words| words.get(&code))?;
        Some(Candidate {
            text: word.to_owned(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        })
    }
}
