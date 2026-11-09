//! 设置里影响候选的几项（含学习语言），与中文键盘的标点键。

use qingjian_core::{Language, NoTranslator, PairEdit, TraditionalVariant};

use super::Session;
use crate::assembly::load_translator;
use crate::frame::Frame;
use crate::options::InputOptions;

impl Session {
    /// 换模糊音、标点全半角、中英混输、学习语言、学习开关、繁体字形、附加词库、分次选词、拼写纠错、上屏后联想这几项；组句中的候选跟着重算。
    pub fn set_options(&mut self, options: &InputOptions) -> Frame {
        self.engine.set_fuzzy(options.fuzzy_rules());
        self.engine
            .set_full_width_punctuation(options.full_width_punctuation);
        self.engine
            .set_pair_punctuation(options.pair_punctuation, options.english_pair_punctuation);
        self.engine
            .set_english_in_chinese(options.english_in_chinese);
        self.engine.set_chinese_first(options.chinese_first);
        self.set_learning_language(options.learning_language());
        self.engine.set_learning(options.learning);
        self.engine.set_hold_selections(options.hold_selections);
        self.engine.set_emoji_limit(usize::from(options.emoji));
        self.engine.set_kaomoji_limit(usize::from(options.kaomoji));
        self.engine.set_symbol_limit(usize::from(options.symbols));
        self.engine
            .set_spelling_correction(options.spelling_correction);
        self.follow_ups = options.follow_ups;
        self.traditional_variant = options.traditional_variant();
        if self.engine.traditional() != TraditionalVariant::Off {
            self.engine.set_traditional(self.traditional_variant);
        }
        self.sync_dictionaries(options);
        self.refresh(String::new())
    }

    /// 学习语言变了才换释义表（每次交设置都走这里）：关是不翻译；没有这门语言的表就保持原样。
    /// 换之前先落盘，旧语言的个人释义表与生词记录不丢。
    fn set_learning_language(&mut self, language: Option<Language>) {
        // 不翻译时 Core 报的学习语言是中文
        let current = Some(self.engine.learning_language()).filter(|l| *l != Language::Chinese);
        if current == language {
            return;
        }
        let translator = match language {
            Some(language) => match load_translator(&self.data_dir, &self.user_dir, language) {
                Some(translator) => translator,
                None => return,
            },
            None => Box::new(NoTranslator),
        };
        self.engine.flush_learning();
        self.engine.set_translator(translator);
        tracing::info!(
            language = language.map_or("off", Language::code),
            "学习语言已切换"
        );
    }

    /// 私密输入（应用声明不要学习的输入框，如浏览器的无痕标签页）：不学习、不记生词与统计、不发云端，同 PC。
    /// 壳每次换输入框时按输入框交一次。
    pub fn set_private(&mut self, private: bool) {
        self.engine.set_private(private);
    }

    /// 中文键盘上的标点键（交进来的是它对应的半角字符）：全角开着按 Core 转成中文标点（引号成对、数字后的点保持半角），
    /// 否则原样；返回要上屏的文字。组句中的输入壳已经先顶了首选。
    /// 成对符号（#928）：键盘上的符号 `text` 要上屏了（中文已转好全角），`english` 是英文键盘。
    /// 返回给壳的编辑：`insert:<退几格>:<文本>` 上屏一对、光标退回；`skip:<右半边>` 光标后正是它就跳过，
    /// 不是就照常上屏；`None` 照常上屏。
    pub fn pair_punctuation(&mut self, text: &str, english: bool) -> Option<String> {
        Some(match self.engine.pair_punctuation(text, english)? {
            PairEdit::Insert { text, back } => format!("insert:{back}:{text}"),
            PairEdit::Skip { closer } => format!("skip:{closer}"),
        })
    }

    /// 没在组句时按了退格：刚补完一对时返回这一对（两个字），壳核对光标前后正是它们再一起删。
    pub fn pair_backspace(&mut self) -> Option<String> {
        self.engine
            .pair_backspace()
            .map(|(open, close)| format!("{open}{close}"))
    }

    pub fn punctuate(&mut self, c: char) -> String {
        match self.engine.punctuate(c) {
            Some(text) => text.to_owned(),
            None => {
                self.engine.note_passthrough(c);
                c.to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use qingjian_core::Engine;
    use qingjian_core::dictionary::Dictionary;

    use super::Session;

    #[test]
    fn pair_edits_are_encoded_for_the_keyboard() {
        let engine = Engine::new(Dictionary::parse("你\tni\t9000\n").unwrap());
        let mut session = Session::new(engine, PathBuf::new(), PathBuf::new(), PathBuf::new());
        let open = session.punctuate('(');
        assert_eq!(
            session.pair_punctuation(&open, false).as_deref(),
            Some("insert:1:（）")
        );
        assert_eq!(session.pair_backspace().as_deref(), Some("（）"));
        session.pair_punctuation("《", false);
        session.punctuate('1');
        assert_eq!(
            session.pair_punctuation("》", false).as_deref(),
            Some("skip:》")
        );
        // 英文缺省不成对
        assert_eq!(session.pair_punctuation("(", true), None);
    }
}
