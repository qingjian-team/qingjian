//! 拼音列里选定一个音节。

use crate::engine::Engine;
use crate::nine_key;

impl Engine {
    /// 第一个没定下来的位置起的数字换成 `alternative` 的字母；后面还有数字就接一个 `'`，
    /// 免得定下来的 `xi` 和后面的 `2` 又拼成 `xia`。写法可以比这个位置短或长（拼音列里的别的切法）。
    /// 写法对不上这里就不动，返回 `false`。
    pub fn pin_nine_key(&mut self, alternative: &str) -> bool {
        let text = self.composition.text().to_owned();
        let (shapes, _) = nine_key::segment(&text);
        let Some((_, position)) = shapes.first().and_then(|shape| shape.first_open()) else {
            return false;
        };
        let start = position.start;
        if !position.alternatives.iter().any(|a| a == alternative)
            && !nine_key::spellings_from(&text, start)
                .iter()
                .any(|a| a == alternative)
        {
            return false;
        }
        let end = start + alternative.len();
        let mut pinned = String::with_capacity(text.len() + 1);
        pinned.push_str(&text[..start]);
        pinned.push_str(alternative);
        if end < text.len() && !text[end..].starts_with('\'') {
            pinned.push('\'');
        }
        pinned.push_str(&text[end..]);
        self.set_input(&pinned);
        true
    }
}
