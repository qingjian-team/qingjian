use super::Unit;
use crate::parser::{Segmentation, Syllable};

/// 一段双拼键解码的结果：能解的单元 + 解不动的尾巴。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decoded {
    /// 解出来的单元，按敲键顺序。
    units: Vec<Unit>,

    /// 从第一个配不出音节的位置起的原始键，原样显示、不参与候选。
    tail: String,

    /// 单元的全拼用 `'` 连成的串（分隔符单元不占位置），直接喂给全拼切分。
    pinyin: String,
}

impl Decoded {
    pub fn new(units: Vec<Unit>, tail: String) -> Self {
        let mut pinyin = String::with_capacity(units.len() * 6);
        for unit in units.iter().filter(|u| !u.is_separator()) {
            if !pinyin.is_empty() {
                pinyin.push('\'');
            }
            pinyin.push_str(&unit.pinyin);
        }
        Self {
            units,
            tail,
            pinyin,
        }
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// 解出来的全拼，音节之间是 `'`。
    pub fn pinyin(&self) -> &str {
        &self.pinyin
    }

    /// 解不动的尾巴（原始键）。
    pub fn tail(&self) -> &str {
        &self.tail
    }

    /// 全部键都解成了完整音节：没有尾巴、末尾没有落单的键。
    pub fn is_complete(&self) -> bool {
        self.tail.is_empty() && self.units.iter().all(|u| u.complete || u.is_separator())
    }

    /// 末尾是不是一个落单的声母键：这时敲 `;`（微软 / 搜狗的 ing）应该进缓冲区而不是当标点。
    pub fn pending_initial(&self) -> bool {
        self.tail.is_empty()
            && self
                .units
                .last()
                .is_some_and(|u| !u.complete && !u.is_separator())
    }

    /// 解出来的唯一切分：双拼两键一音节没有歧义，不必再走全拼切分（那会把 `zhong` 再拆出 `z… hong` 一类简拼切法）。
    /// 落单的键是残缺音节。空的（一个单元都没解出）返回 `None`。
    pub fn segmentation(&self) -> Option<Segmentation> {
        let syllables: Vec<Syllable> = self
            .units
            .iter()
            .filter(|u| !u.is_separator())
            .map(|u| {
                if u.complete {
                    Syllable::complete(&u.pinyin)
                } else {
                    Syllable::partial(&u.pinyin)
                }
            })
            .collect();
        (!syllables.is_empty()).then_some(Segmentation { syllables })
    }

    /// 显示形式：全拼接上尾巴（`ni'bl`）。
    pub fn marked(&self) -> String {
        match (self.pinyin.is_empty(), self.tail.is_empty()) {
            (_, true) => self.pinyin.clone(),
            (true, false) => self.tail.clone(),
            (false, false) => format!("{}'{}", self.pinyin, self.tail),
        }
    }

    /// [`Self::pinyin`] 开头 `pinyin_len` 个字节对应多少个键：整单元被盖住才算，紧跟其后的 `'` 一并算上。
    pub fn keys_for(&self, pinyin_len: usize) -> usize {
        let mut keys = 0;
        let mut position = 0;
        let mut first = true;
        let mut pending_separators = 0;
        for unit in &self.units {
            if unit.is_separator() {
                pending_separators += unit.keys.len();
                continue;
            }
            let start = if first { 0 } else { position + 1 };
            let end = start + unit.pinyin.len();
            if pinyin_len < end {
                break;
            }
            keys += pending_separators + unit.keys.len();
            pending_separators = 0;
            position = end;
            first = false;
        }
        // 消耗到末尾时，后面紧跟的 `'` 也一起吃掉
        if keys > 0 {
            keys += pending_separators;
        }
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::super::Scheme;

    #[test]
    fn keys_follow_consumed_syllables() {
        let decoded = Scheme::Xiaohe.decode("kdfave");
        assert_eq!(decoded.pinyin(), "kai'fa'zhe");
        assert_eq!(decoded.keys_for(0), 0);
        assert_eq!(decoded.keys_for(3), 2);
        assert_eq!(decoded.keys_for(6), 4);
        assert_eq!(decoded.keys_for(10), 6);
        // 只盖住半个音节不算
        assert_eq!(decoded.keys_for(5), 2);
    }

    #[test]
    fn partial_last_unit_counts_one_key() {
        let decoded = Scheme::Xiaohe.decode("kdf");
        assert_eq!(decoded.keys_for(5), 3);
        assert_eq!(decoded.keys_for(3), 2);
    }

    #[test]
    fn separators_go_with_the_syllable_before_them() {
        let decoded = Scheme::Xiaohe.decode("xi'an");
        assert_eq!(decoded.pinyin(), "xi'an");
        assert_eq!(decoded.keys_for(2), 3);
        assert_eq!(decoded.keys_for(5), 5);
        assert!(decoded.is_complete());
    }

    #[test]
    fn segmentation_mirrors_units() {
        let decoded = Scheme::Xiaohe.decode("kdf'ni");
        let segmentation = decoded.segmentation().unwrap();
        assert_eq!(segmentation.to_string(), "kai f… ni");
        assert!(Scheme::Xiaohe.decode("bl").segmentation().is_none());
    }

    #[test]
    fn pending_initial_wants_a_final() {
        assert!(Scheme::Microsoft.decode("x").pending_initial());
        assert!(Scheme::Microsoft.decode("nix").pending_initial());
        assert!(!Scheme::Microsoft.decode("ni").pending_initial());
        assert!(!Scheme::Microsoft.decode("").pending_initial());
        assert!(!Scheme::Microsoft.decode("nibl").pending_initial());
    }
}
