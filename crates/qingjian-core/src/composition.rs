//! 输入状态机：拼音缓冲区与光标。
//!
//! 只维护「用户已经敲了什么、光标在哪」，不理解拼音，也不知道候选。
//! 缓冲区只含 ASCII 小写字母和 `'`（表达式模式下还有数字与运算符，见 `shortcut`；微软 / 搜狗双拼下还有 `;`），所以字节下标即字符下标。

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Composition {
    /// 用户已敲入、尚未上屏的拼音，统一小写。
    buffer: String,

    /// 光标位置（字节下标，`0..=buffer.len()`），插入和退格都相对它。
    cursor: usize,
}

impl Composition {
    pub fn text(&self) -> &str {
        &self.buffer
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// 参与候选生成的那段拼音：光标停在中间时只算光标前的（`ni|hao` → `ni`），
    /// 光标在开头或末尾时算整段。搜狗的行为：往回挪光标就是要先把前半段上屏。
    pub fn scope(&self) -> &str {
        if self.cursor == 0 || self.cursor >= self.buffer.len() {
            &self.buffer
        } else {
            &self.buffer[..self.cursor]
        }
    }

    /// 作用域之后剩下的拼音（`ni|hao` → `hao`），只用于显示。
    pub fn rest(&self) -> &str {
        &self.buffer[self.scope().len()..]
    }

    /// 在光标处插入一个字符。
    pub fn push(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// 删掉光标前一个字符。光标在开头时返回 `false`。
    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.buffer.remove(self.cursor);
        true
    }

    /// 删掉光标后一个字符。光标在末尾时返回 `false`。
    pub fn delete_forward(&mut self) -> bool {
        if self.cursor >= self.buffer.len() {
            return false;
        }
        self.buffer.remove(self.cursor);
        true
    }

    /// 光标左移一格。已在开头时返回 `false`。
    pub fn move_left(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    /// 光标右移一格。已在末尾时返回 `false`。
    pub fn move_right(&mut self) -> bool {
        if self.cursor >= self.buffer.len() {
            return false;
        }
        self.cursor += 1;
        true
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    /// 删掉开头 `len` 个字节（上屏消耗掉的拼音），光标随之前移；
    /// 被吃掉的部分盖过了光标（`ni|hao` 上屏了 你）时光标落到末尾，接着组句就是往后打。
    pub fn drain_prefix(&mut self, len: usize) {
        let len = len.min(self.buffer.len());
        self.buffer.drain(..len);
        self.cursor = if self.cursor > len {
            self.cursor - len
        } else {
            self.buffer.len()
        };
    }

    /// 吃掉整个作用域（云端词 / 英文词 / 整句对应的是整个作用域）。
    pub fn drain_scope(&mut self) {
        let len = self.scope().len();
        self.drain_prefix(len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(text: &str) -> Composition {
        let mut composition = Composition::default();
        for c in text.chars() {
            composition.push(c);
        }
        composition
    }

    #[test]
    fn push_and_backspace() {
        let mut composition = Composition::default();
        // 原样保留大小写（英文直输段 `no-Way` 要用），中文模式下壳只会送小写
        composition.push('K');
        composition.push('a');
        assert_eq!(composition.text(), "Ka");
        assert_eq!(composition.cursor(), 2);
        assert!(composition.backspace());
        assert!(composition.backspace());
        assert!(!composition.backspace());
        assert!(composition.is_empty());
    }

    #[test]
    fn edits_happen_at_the_cursor() {
        let mut composition = typed("kaifa");
        assert!(composition.move_left());
        assert!(composition.move_left());
        composition.push('n');
        assert_eq!(composition.text(), "kainfa");
        assert_eq!(composition.cursor(), 4);
        assert!(composition.backspace());
        assert_eq!(composition.text(), "kaifa");
        assert!(composition.delete_forward());
        assert_eq!(composition.text(), "kaia");
        composition.move_home();
        assert!(!composition.move_left());
        assert!(!composition.backspace());
        composition.move_end();
        assert!(!composition.move_right());
        assert!(!composition.delete_forward());
    }

    #[test]
    fn scope_is_text_before_cursor_unless_cursor_is_at_either_end() {
        let mut composition = typed("nihao");
        assert_eq!(composition.scope(), "nihao");
        assert_eq!(composition.rest(), "");
        composition.move_left();
        composition.move_left();
        composition.move_left();
        assert_eq!(composition.scope(), "ni");
        assert_eq!(composition.rest(), "hao");
        composition.move_home();
        assert_eq!(composition.scope(), "nihao");
        assert_eq!(composition.rest(), "");
    }

    #[test]
    fn draining_past_the_cursor_moves_it_to_the_end() {
        let mut composition = typed("nihao");
        for _ in 0..3 {
            composition.move_left();
        }
        composition.drain_scope();
        assert_eq!(composition.text(), "hao");
        assert_eq!(composition.cursor(), 3);
    }

    #[test]
    fn drain_prefix_moves_cursor_back() {
        let mut composition = typed("kaifazhe");
        composition.move_left();
        composition.drain_prefix(5);
        assert_eq!(composition.text(), "zhe");
        assert_eq!(composition.cursor(), 2);
        composition.drain_prefix(10);
        assert!(composition.is_empty());
        assert_eq!(composition.cursor(), 0);
    }
}
