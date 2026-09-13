//! 附加候选：日期时间等快捷项、中英混输的英文词与补全、emoji。

use super::*;

impl Engine {
    /// 精确匹配自定义输入码时，数字键应选择候选。
    pub(super) fn has_custom_phrase(&self) -> bool {
        !self.english_mode
            && self
                .custom_phrases
                .iter()
                .any(|p| p.enabled && p.code == self.composition.scope())
    }

    /// 所有普通候选完成排序后按输入码固定位置。
    pub(super) fn insert_custom_phrases(&self, items: &mut Vec<Candidate>) {
        if self.english_mode {
            return;
        }
        let mut phrases: Vec<_> = self
            .custom_phrases
            .iter()
            .filter(|p| p.enabled && p.code == self.composition.scope())
            .collect();
        phrases.sort_by_key(|p| p.position);
        for phrase in phrases {
            while items.len() < phrase.position - 1 {
                items.push(Candidate {
                    text: String::new(),
                    kind: CandidateKind::Custom(0),
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                });
            }
            items.insert(
                phrase.position - 1,
                Candidate {
                    text: phrase.text.clone(),
                    kind: CandidateKind::Custom(phrase.position),
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                },
            );
        }
    }

    /// 日期 / 时间 / 星期这类快捷候选插在本地首选之后：`rq` 首选仍是词库里的词，快捷写法紧随其后。
    pub(super) fn insert_shortcuts(&self, items: &mut Vec<Candidate>, scope: &str) {
        let shortcuts = shortcut::candidates(scope, self.modes().expression, &jiff::Zoned::now());
        if shortcuts.is_empty() {
            return;
        }
        let position = items.len().min(1);
        items.splice(position..position, shortcuts);
    }

    /// 中英混输：整段输入是英文词就把它加进候选。
    /// 作为拼音「不像话」（切不动、或除末尾外还有声母缩写 / 残缺音节）时排第一，否则排第二。
    pub(super) fn insert_english(&self, items: &mut Vec<Candidate>, unlikely_pinyin: bool) {
        let lists = self.english_lists();
        if lists.is_empty() {
            return;
        }
        let text = self.composition.scope();
        if text.contains('\'') {
            return;
        }
        let english_candidate = |word: &str| Candidate {
            text: word.to_owned(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
        };
        let word = lists.iter().find_map(|words| words.get(text));
        // 这段字母下用户选中文词（`key` → 可以）比选英文词的次数多：中文词留在第一，英文让到后面；
        // 拼音再不像话也是他自己教的
        let chosen = items
            .first()
            .filter(|c| c.kind == CandidateKind::Chinese)
            .map_or(0, |c| self.learner.choice_weight(text, &c.text));
        let english_weight = word.map_or(0, |w| self.learner.weight(w));
        let mut position = if items.is_empty() || (unlikely_pinyin && chosen <= english_weight) {
            0
        } else {
            1
        };
        if let Some(word) = word {
            items.insert(position, english_candidate(word));
            position += 1;
        }
        // 英文补全：拼音不像话时（`compa` 切成 co'm'pa），整段多半是在打英文词的前面几个字母，补全紧跟在精确词之后；
        // 个人词表在前，两张表里都有的只出一次
        if unlikely_pinyin && text.len() >= MIN_COMPLETION_LETTERS {
            let mut budget = ENGLISH_COMPLETIONS;
            for words in &lists {
                for word in words.complete(text, budget) {
                    if items
                        .iter()
                        .any(|c| c.kind == CandidateKind::English && c.text == word)
                    {
                        continue;
                    }
                    items.insert(position, english_candidate(word));
                    position += 1;
                    budget -= 1;
                }
                if budget == 0 {
                    break;
                }
            }
        }
    }

    /// 给英文候选用的词表，个人的在前、随包的在后；一张都没有就是空。
    pub(super) fn english_lists(&self) -> Vec<&WordList> {
        self.learner
            .user_english()
            .into_iter()
            .chain(self.english.as_ref())
            .collect()
    }

    /// emoji 候选：前几个中文候选里有配 emoji 的，emoji 紧跟在那个词后面，右侧标注它对应的词。
    pub(super) fn insert_emoji(&self, items: &mut Vec<Candidate>) {
        let Some(table) = &self.emoji else { return };
        let mut inserted = 0;
        let mut index = 0;
        let mut scanned = 0;
        while index < items.len() && scanned < EMOJI_SCAN && inserted < EMOJI_TOTAL {
            let item = &items[index];
            index += 1;
            if !matches!(
                item.kind,
                CandidateKind::Chinese | CandidateKind::Sentence | CandidateKind::English
            ) {
                continue;
            }
            scanned += 1;
            // 英文词按小写查英文表（smile → 😀）
            let key = if item.kind == CandidateKind::English {
                item.text.to_lowercase()
            } else {
                item.text.clone()
            };
            let emojis = table.lookup(&key);
            if emojis.is_empty() {
                continue;
            }
            let word = item.text.clone();
            let syllables = item.syllables.clone();
            for emoji in emojis.iter().take(EMOJI_PER_WORD) {
                if inserted >= EMOJI_TOTAL {
                    break;
                }
                items.insert(
                    index,
                    Candidate {
                        text: emoji.clone(),
                        kind: CandidateKind::Emoji,
                        syllables: syllables.clone(),
                        reading: Some(word.clone()),
                        translation: None,
                    },
                );
                index += 1;
                inserted += 1;
            }
        }
    }
}
