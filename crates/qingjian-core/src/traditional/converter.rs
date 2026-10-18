//! 简繁转换器。

use std::cell::RefCell;
use std::collections::HashMap;

use ferrous_opencc::OpenCC;
use ferrous_opencc::config::BuiltinConfig;

use super::TraditionalVariant;
use crate::candidate::{CandidateKind, CandidateList, Language, Translation};

/// 简繁转换。
///
/// **只在 Core 的出口转**：词库、词频、用户词、个人 n-gram、输入日志里一律是简体。
/// [`Self::apply`] 把一轮候选换成繁体并记下「繁体 → 简体」，壳原样显示、原样把候选送回来上屏；
/// [`Self::restore`] 按这张表换回简体，于是学习、历史、日志那一整套照旧看到简体。
///
/// 不把繁体直接写进学习数据有两个原因：反向转换不可靠（一个繁体字常对应多个简体字），
/// 而且用户改回简体输出后，之前学到的繁体词就再也匹配不上了。
pub struct Traditional {
    /// 配置里选的变体，转换器没建起来时也是它；实际有没有在转看 [`Self::is_on`]。
    variant: TraditionalVariant,

    /// OpenCC 转换器；变体为 [`TraditionalVariant::Off`] 或字典建不起来时为 `None`。
    converter: Option<OpenCC>,

    /// 上一次 [`Self::apply`] 换过的候选：繁体文本 → 简体原文，只记两者不同的。
    restore: RefCell<HashMap<String, String>>,
}

impl Default for Traditional {
    fn default() -> Self {
        Self::new(TraditionalVariant::Off)
    }
}

impl Traditional {
    /// 按变体建转换器。字典建不起来只记日志并退回简体输出：输入法不能因为转换没就绪就打不出字。
    pub fn new(variant: TraditionalVariant) -> Self {
        let config = match variant {
            TraditionalVariant::Off => None,
            TraditionalVariant::Taiwan => Some(BuiltinConfig::S2tw),
            TraditionalVariant::Hongkong => Some(BuiltinConfig::S2hk),
            TraditionalVariant::Standard => Some(BuiltinConfig::S2t),
        };
        let converter = config.and_then(|config| match OpenCC::from_config(config) {
            Ok(converter) => Some(converter),
            Err(error) => {
                tracing::error!(%error, ?variant, "简繁转换字典建不起来，按简体输出");
                None
            }
        });
        Self {
            variant,
            converter,
            restore: RefCell::new(HashMap::new()),
        }
    }

    /// 配置里选的变体。
    pub fn variant(&self) -> TraditionalVariant {
        self.variant
    }

    /// 真的在转吗：变体不是 [`TraditionalVariant::Off`] 且字典建起来了。
    pub fn is_on(&self) -> bool {
        self.converter.is_some()
    }

    /// 把一轮候选换成繁体，并记下「繁体 → 简体」。没开转换时什么都不做。
    /// 自定义短语不转：它是用户写下的原文，原样上屏（与自动空格跳过它是同一条规矩）。
    ///
    /// 表不在这里清：云端联想词是在 [`crate::Engine::poll_prediction`] 那条路上转的，
    /// 每轮查询清一次会把它的映射冲掉，之后上屏就还原不回简体。清表在组句边界，见 [`Self::forget`]。
    pub fn apply(&self, list: &mut CandidateList) {
        let Some(converter) = &self.converter else {
            return;
        };
        let mut restore = self.restore.borrow_mut();
        for item in &mut list.items {
            if matches!(item.kind, CandidateKind::Custom(_)) {
                continue;
            }
            // emoji、颜文字、符号的 reading 是它跟着的那个中文词（🧑‍💻 跟着 软件），也要换，
            // 不然一屏里混两种字形。中文候选不用 reading，日文假名不是这几类，碰不到
            if item.kind.follows_word()
                && let Some(reading) = &mut item.reading
            {
                *reading = converter.convert(reading);
            }
            if let Some(translation) = &mut item.translation {
                convert_chinese_senses(converter, translation);
            }
            let converted = converter.convert(&item.text);
            if converted != item.text {
                let simplified = std::mem::replace(&mut item.text, converted.clone());
                restore.insert(converted, simplified);
            }
        }
    }

    /// 转中文译词：英文候选的中文释义走这条（它在 [`crate::Engine::annotate`] 里才挂上，赶不上 [`Self::apply`]）。
    /// 学习语言是英语 / 日语时原样不动。
    pub fn apply_translation(&self, translation: &mut Translation) {
        if let Some(converter) = &self.converter {
            convert_chinese_senses(converter, translation);
        }
    }

    /// 转一段文本并记下「繁体 → 简体」，给不走候选列表的那条路用（云端联想词与整句，
    /// 它们在 [`crate::Engine::poll_prediction`] 里出 Core，赶不上 [`Self::apply`]）。
    /// 没开转换时原样返回、不记表。
    pub fn convert_remember(&self, text: &str) -> String {
        let Some(converter) = &self.converter else {
            return text.to_owned();
        };
        let converted = converter.convert(text);
        if converted != text {
            self.restore
                .borrow_mut()
                .insert(converted.clone(), text.to_owned());
        }
        converted
    }

    /// 清掉还原表，组句边界调用（这一段的候选与联想词都作废了）。
    pub fn forget(&self) {
        self.restore.borrow_mut().clear();
    }

    /// 繁体候选对应的简体原文；这一轮没换过它（或压根没开转换）时为 `None`。
    pub fn restore(&self, text: &str) -> Option<String> {
        self.restore.borrow().get(text).cloned()
    }

    /// 单独转一段文本。没开转换时原样返回。
    pub fn convert(&self, text: &str) -> String {
        match &self.converter {
            Some(converter) => converter.convert(text),
            None => text.to_owned(),
        }
    }
}

/// 把中文释义换成繁体。学习语言是英语 / 日语时不动：那些译词不是中文，简繁转换碰了只会出错。
fn convert_chinese_senses(converter: &OpenCC, translation: &mut Translation) {
    if translation.language != Language::Chinese {
        return;
    }
    for sense in translation.senses_mut() {
        sense.text = converter.convert(&sense.text);
    }
}

impl std::fmt::Debug for Traditional {
    /// `OpenCC` 没有 `Debug`，而 [`crate::Engine`] 要能打印。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Traditional")
            .field("variant", &self.variant)
            .field("on", &self.is_on())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::{Candidate, Sense};

    fn list(texts: &[&str]) -> CandidateList {
        CandidateList {
            items: texts
                .iter()
                .map(|text| Candidate {
                    text: (*text).to_owned(),
                    kind: CandidateKind::Chinese,
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                    aux_code: None,
                })
                .collect(),
        }
    }

    /// 关着的时候一个字都不动，也不记还原表。
    #[test]
    fn off_leaves_candidates_alone() {
        let traditional = Traditional::new(TraditionalVariant::Off);
        assert!(!traditional.is_on());
        let mut candidates = list(&["软件", "中华"]);
        traditional.apply(&mut candidates);
        assert_eq!(candidates.items[0].text, "软件");
        assert_eq!(traditional.restore("軟件"), None);
        assert_eq!(traditional.convert("软件"), "软件");
    }

    /// 台湾那档只换成台湾字形、不换用语（与 0.2.1 的 `traditional = true` 相同）：软件 → 軟件，里 → 裡。
    #[test]
    fn taiwan_uses_taiwan_glyphs_without_changing_vocabulary() {
        let traditional = Traditional::new(TraditionalVariant::Taiwan);
        assert!(traditional.is_on());
        assert_eq!(traditional.convert("软件"), "軟件");
        assert_eq!(traditional.convert("这里"), "這裡");
    }

    /// 通用那档只换字形，用语照旧；字形与台湾那档不同（裏 / 裡）。
    #[test]
    fn standard_keeps_vocabulary() {
        let traditional = Traditional::new(TraditionalVariant::Standard);
        assert_eq!(traditional.convert("软件"), "軟件");
        assert_eq!(traditional.convert("这里"), "這裏");
    }

    /// 换过的候选能按繁体文本找回简体原文，没换过的返回 `None`。
    #[test]
    fn restores_converted_candidates_to_simplified() {
        let traditional = Traditional::new(TraditionalVariant::Taiwan);
        // 你好 两种写法一样，不进还原表；上屏时按原样走，学习照旧
        let mut candidates = list(&["软件", "你好"]);
        traditional.apply(&mut candidates);
        assert_eq!(candidates.items[0].text, "軟件");
        assert_eq!(candidates.items[1].text, "你好");
        assert_eq!(traditional.restore("軟件").as_deref(), Some("软件"));
        assert_eq!(traditional.restore("你好"), None);
    }

    /// emoji 跟着的那个中文词、以及中文释义都要一起换，免得一屏里混两种字形；
    /// 英语那份译词不是中文，一个字都不能动。
    #[test]
    fn emoji_reading_and_chinese_senses_are_converted() {
        let traditional = Traditional::new(TraditionalVariant::Taiwan);
        let sense = |text: &str| Sense {
            part_of_speech: None,
            text: text.to_owned(),
            reading: None,
            fresh: false,
        };
        let mut candidates = CandidateList {
            items: vec![
                Candidate {
                    text: "🧑‍💻".to_owned(),
                    kind: CandidateKind::Emoji,
                    syllables: Vec::new(),
                    reading: Some("软件".to_owned()),
                    translation: None,
                    aux_code: None,
                },
                Candidate {
                    text: "软件".to_owned(),
                    kind: CandidateKind::English,
                    syllables: Vec::new(),
                    reading: None,
                    translation: Some(Translation::new(Language::Chinese, vec![sense("软件工程")])),
                    aux_code: None,
                },
                Candidate {
                    text: "开发".to_owned(),
                    kind: CandidateKind::Chinese,
                    syllables: Vec::new(),
                    reading: None,
                    translation: Some(Translation::new(Language::English, vec![sense("软件")])),
                    aux_code: None,
                },
            ],
        };
        traditional.apply(&mut candidates);
        assert_eq!(candidates.items[0].reading.as_deref(), Some("軟件"));
        assert_eq!(
            candidates.items[1].translation.as_ref().unwrap().senses()[0].text,
            "軟件工程"
        );
        // 英语那条的语言不是中文：原样留着
        assert_eq!(
            candidates.items[2].translation.as_ref().unwrap().senses()[0].text,
            "软件"
        );
    }

    /// 还原表跨轮累积，清表在组句边界：同一段组句里云端词可能在上一轮转的，这一轮上屏还要还原得回去。
    #[test]
    fn restore_table_spans_rounds_until_forget() {
        let traditional = Traditional::new(TraditionalVariant::Taiwan);
        traditional.apply(&mut list(&["软件"]));
        traditional.apply(&mut list(&["内存"]));
        assert_eq!(traditional.restore("軟件").as_deref(), Some("软件"));
        assert_eq!(traditional.restore("內存").as_deref(), Some("内存"));

        traditional.forget();
        assert_eq!(traditional.restore("軟件"), None);
        assert_eq!(traditional.restore("內存"), None);
    }

    /// 不走候选列表的那条路：转了也要记表，还原得回去。
    #[test]
    fn convert_remember_records_the_mapping() {
        let traditional = Traditional::new(TraditionalVariant::Taiwan);
        assert_eq!(traditional.convert_remember("软件"), "軟件");
        assert_eq!(traditional.restore("軟件").as_deref(), Some("软件"));
    }

    /// 自定义短语是用户写的原文：开着繁体也原样上屏，不进还原表。
    #[test]
    fn custom_phrases_stay_verbatim() {
        let traditional = Traditional::new(TraditionalVariant::Taiwan);
        let mut candidates = list(&["软件园"]);
        candidates.items[0].kind = CandidateKind::Custom(1);
        traditional.apply(&mut candidates);
        assert_eq!(candidates.items[0].text, "软件园");
        assert_eq!(traditional.restore("軟件園"), None);
    }
}
