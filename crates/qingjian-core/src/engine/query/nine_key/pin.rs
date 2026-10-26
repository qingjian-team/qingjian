//! 拼音列里选定一个音节。

use crate::engine::Engine;
use crate::nine_key;

impl Engine {
    /// 第一个没定下来的位置上的数字换成 `alternative` 的字母；不是末尾的位置后面接一个 `'`，
    /// 免得定下来的 `xi` 和后面的 `2` 又拼成 `xia`。写法对不上这个位置就不动，返回 `false`。
    pub fn pin_nine_key(&mut self, alternative: &str) -> bool {
        let text = self.composition.text().to_owned();
        let (shapes, _) = nine_key::segment(&text);
        let Some((index, position)) = shapes.first().and_then(|shape| shape.first_open()) else {
            return false;
        };
        if !position.alternatives.iter().any(|a| a == alternative) {
            return false;
        }
        let end = position.start + position.len;
        let last = index + 1 == shapes[0].positions.len();
        let mut pinned = String::with_capacity(text.len() + 1);
        pinned.push_str(&text[..position.start]);
        pinned.push_str(alternative);
        if !last && !text[end..].starts_with('\'') {
            pinned.push('\'');
        }
        pinned.push_str(&text[end..]);
        self.set_input(&pinned);
        true
    }
}
