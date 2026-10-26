//! 回放期间按日志里记的方案装配引擎。
//!
//! 每条上屏的 `scheme` 字段写着当时用的是哪套方案（双拼的键、`zhuyin`、形码方案的键、全拼为空，
//! 混输是 `<拼音侧>+<形码>`），回放要照着还原，否则拿拼音的读法去喂形码的键、或者反过来，
//! 算出来的命中率没有意义。整份日志通常只有一套方案，所以只在方案串变化时才动码表——
//! `Engine::set_code_scheme` 收所有权，换一次要克隆一份 8.9 万条的码表。

use std::sync::Arc;

use qingjian_core::{CodeScheme, Engine, ShuangpinLayout};
use qingjian_platform::Scheme;

/// 按日志里的方案串装配引擎，并记住当前装的是哪套。
pub struct SchemeSwitcher {
    /// 当前已装配的方案串；没装过是 `None`。
    current: Option<String>,

    /// 日志里出现形码时用的方案；没给 `--code` 就是 `None`，那部分日志回放不了。
    /// 只有一套：日志里不管记的是哪个形码，都拿它回放。
    code: Option<CodeScheme>,

    /// 日志里出现用户双拼方案时用的键位（`--shuangpin` 给的方案目录）；日志里方案键与它的 id 相同才认。
    shuangpin: Option<Arc<ShuangpinLayout>>,
}

impl SchemeSwitcher {
    pub fn new(code: Option<CodeScheme>, shuangpin: Option<Arc<ShuangpinLayout>>) -> Self {
        Self {
            current: None,
            code,
            shuangpin,
        }
    }

    /// 这条日志能不能回放：带形码的要有码表，别的方案都行。
    pub fn can_replay(&self, key: &str) -> bool {
        self.split(key).1.is_none() || self.code.is_some()
    }

    /// 按 `key` 装配引擎。双拼与注音的开关每次都设（便宜）；码表只在方案串变了时换。
    pub fn apply(&mut self, engine: &mut Engine, key: &str) {
        let (pinyin, code) = self.split(key);
        match pinyin {
            Scheme::Custom => engine.set_shuangpin_layout(self.shuangpin.clone()),
            _ => engine.set_shuangpin(pinyin.shuangpin()),
        }
        engine.set_zhuyin_mode(pinyin == Scheme::Zhuyin);
        engine.set_phonetic(pinyin.is_on());
        if self.current.as_deref() == Some(key) {
            return;
        }
        self.current = Some(key.to_owned());
        engine.set_code_scheme(code.and(self.code.clone()));
    }

    fn split<'a>(&self, key: &'a str) -> (Scheme, Option<&'a str>) {
        split(key, self.shuangpin.as_deref().map(ShuangpinLayout::key))
    }
}

/// 日志里的方案串拆成「拼音侧方案 + 形码方案的键」：
/// `"xiaohe+wubi"` → 小鹤 + `wubi`；`"huma"` → 拼音关 + `huma`；`""` → 全拼 + 关。
/// 空串与认不出来的写法都按全拼（老日志里没有这个字段）。
/// 用户双拼方案与形码方案的键长得一样，只有与 `custom`（`--shuangpin` 给的方案 id）相同的才当双拼。
fn split<'a>(key: &'a str, custom: Option<&str>) -> (Scheme, Option<&'a str>) {
    let pinyin = |side: &str| match side.parse() {
        _ if Some(side) == custom => Scheme::Custom,
        Ok(scheme) => scheme,
        Err(_) => Scheme::Pinyin,
    };
    if let Some((side, code)) = key.split_once('+') {
        return (pinyin(side), Some(code));
    }
    if Some(key) == custom {
        return (Scheme::Custom, None);
    }
    // `wubi` 在拼音侧是老配置的写法（解成「关」），在日志里是只用 86 五笔
    if key == "wubi" {
        return (Scheme::Off, Some(key));
    }
    match key.parse() {
        Ok(pinyin) => (pinyin, None),
        Err(_) if qingjian_platform::code_schemes::is_valid_id(key) => (Scheme::Off, Some(key)),
        Err(_) => (Scheme::Pinyin, None),
    }
}

#[cfg(test)]
mod tests {
    use qingjian_dictionary::CodeTable;

    use super::*;

    fn wubi() -> CodeScheme {
        CodeScheme::new("wubi", CodeTable::parse("一\tggll\t100\n").unwrap())
    }

    #[test]
    fn recognizes_which_schemes_need_a_table() {
        let without = SchemeSwitcher::new(None, None);
        assert!(without.can_replay(""));
        assert!(without.can_replay("xiaohe"));
        assert!(without.can_replay("zhuyin"));
        assert!(!without.can_replay("wubi"));
        assert!(!without.can_replay("huma"));

        assert!(SchemeSwitcher::new(Some(wubi()), None).can_replay("wubi"));
    }

    #[test]
    fn applying_a_code_scheme_hands_the_table_to_the_engine() {
        use qingjian_dictionary::Dictionary;

        let mut switcher = SchemeSwitcher::new(Some(wubi()), None);
        let mut engine = Engine::new(Dictionary::parse("开\tkai\t100\n").unwrap());

        switcher.apply(&mut engine, "wubi");
        engine.set_input("ggll");
        assert_eq!(engine.query().unwrap().candidates.items[0].text, "一");

        // 切回全拼：码表卸掉，同一串编码不再出中文候选
        switcher.apply(&mut engine, "");
        engine.set_input("ggll");
        let texts: Vec<String> = engine
            .query()
            .map(|q| q.candidates.items.into_iter().map(|c| c.text).collect())
            .unwrap_or_default();
        assert!(!texts.contains(&"一".to_owned()));
    }

    #[test]
    fn scheme_strings_split_into_the_two_axes() {
        // 老日志没有 scheme 字段，解出来是全拼、不开形码
        assert_eq!(split("", None), (Scheme::Pinyin, None));
        assert_eq!(split("没见过的写法", None), (Scheme::Pinyin, None));
        assert_eq!(
            split("xiaohe", None),
            (
                Scheme::Shuangpin(qingjian_core::ShuangpinScheme::Xiaohe),
                None
            )
        );
        // 只用形码
        assert_eq!(split("wubi", None), (Scheme::Off, Some("wubi")));
        assert_eq!(split("huma", None), (Scheme::Off, Some("huma")));
        // 混输：拼音侧那部分照解，形码跟着开
        assert_eq!(
            split("xiaohe+wubi", None),
            (
                Scheme::Shuangpin(qingjian_core::ShuangpinScheme::Xiaohe),
                Some("wubi")
            )
        );
        assert_eq!(split("pinyin+wubi", None), (Scheme::Pinyin, Some("wubi")));
        assert_eq!(split("zhuyin+huma", None), (Scheme::Zhuyin, Some("huma")));
        assert_eq!(split("mine", Some("mine")), (Scheme::Custom, None));
        assert_eq!(
            split("mine+huma", Some("mine")),
            (Scheme::Custom, Some("huma"))
        );
        assert_eq!(split("huma", Some("mine")), (Scheme::Off, Some("huma")));
    }
}
