use std::sync::LazyLock;

use super::syllable::{MAX_SYLLABLE_LEN, SYLLABLES};

/// 全局音节 trie，首次使用时从 [`SYLLABLES`] 构建。
pub static SYLLABLE_TRIE: LazyLock<SyllableTrie> = LazyLock::new(|| SyllableTrie::build(SYLLABLES));

/// trie 节点。子节点按字母 a–z 索引，0 表示无子节点（根节点永远是 0 号，不会被引用为子节点）。
#[derive(Debug, Clone)]
struct Node {
    /// 26 个字母对应的子节点下标。
    children: [u16; 26],

    /// 从根到此节点的路径是否为一个完整音节。
    terminal: bool,
}

impl Node {
    const EMPTY: Self = Self {
        children: [0; 26],
        terminal: false,
    };
}

/// 从输入某个位置起的匹配结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matches {
    /// 以该位置开头的合法音节长度，升序；最多 [`MAX_SYLLABLE_LEN`] 个。
    lengths: [u8; MAX_SYLLABLE_LEN],

    /// `lengths` 里有效的个数。
    count: u8,

    /// 剩余整段是某个更长音节的真前缀（用于末尾残缺音节）。
    pub whole_is_prefix: bool,
}

impl Matches {
    pub fn lengths(&self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.lengths[..usize::from(self.count)]
            .iter()
            .map(|&len| usize::from(len))
    }
}

/// 小写字母 trie，只接受 a–z。
#[derive(Debug)]
pub struct SyllableTrie {
    /// 节点池，0 号为根。
    nodes: Vec<Node>,
}

impl SyllableTrie {
    pub fn build(syllables: &[&str]) -> Self {
        let mut trie = Self {
            nodes: vec![Node::EMPTY],
        };
        for syllable in syllables {
            let mut current = 0usize;
            for byte in syllable.bytes() {
                let slot = Self::slot(byte).expect("音节表只能含小写字母");
                let next = trie.nodes[current].children[slot];
                current = if next == 0 {
                    trie.nodes.push(Node::EMPTY);
                    let index = trie.nodes.len() - 1;
                    trie.nodes[current].children[slot] =
                        u16::try_from(index).expect("音节 trie 节点数不会超过 u16");
                    index
                } else {
                    usize::from(next)
                };
            }
            trie.nodes[current].terminal = true;
        }
        trie
    }

    fn slot(byte: u8) -> Option<usize> {
        byte.is_ascii_lowercase().then(|| usize::from(byte - b'a'))
    }

    /// 沿 `rest` 从根走到走不动为止，收集途中经过的完整音节。
    pub fn matches(&self, rest: &str) -> Matches {
        let mut result = Matches {
            lengths: [0; MAX_SYLLABLE_LEN],
            count: 0,
            whole_is_prefix: false,
        };
        let mut current = 0usize;
        for (index, byte) in rest.bytes().enumerate() {
            let Some(slot) = Self::slot(byte) else {
                return result;
            };
            let next = self.nodes[current].children[slot];
            if next == 0 {
                return result;
            }
            current = usize::from(next);
            if self.nodes[current].terminal {
                result.lengths[usize::from(result.count)] =
                    u8::try_from(index + 1).expect("音节长度不超过 6");
                result.count += 1;
            }
        }
        // 整段走完了还有子节点，说明它是某个更长音节的真前缀
        result.whole_is_prefix =
            !rest.is_empty() && self.nodes[current].children.iter().any(|&c| c != 0);
        result
    }

    pub fn contains(&self, s: &str) -> bool {
        self.matches(s).lengths().next_back() == Some(s.len()) && !s.is_empty()
    }

    /// `s` 是否为某个合法音节的**真**前缀（不包含它自己就是完整音节的情况）。
    pub fn is_proper_prefix(&self, s: &str) -> bool {
        self.matches(s).whole_is_prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_matches_table_exactly() {
        for syllable in SYLLABLES {
            assert!(SYLLABLE_TRIE.contains(syllable), "{syllable}");
        }
        assert!(!SYLLABLE_TRIE.contains(""));
        assert!(!SYLLABLE_TRIE.contains("v"));
        assert!(!SYLLABLE_TRIE.contains("kaif"));
        assert!(!SYLLABLE_TRIE.contains("Kai"));
    }

    #[test]
    fn matches_lists_all_syllable_lengths_ascending() {
        let lengths: Vec<usize> = SYLLABLE_TRIE.matches("xiangmu").lengths().collect();
        assert_eq!(lengths, [2, 3, 4, 5]); // xi / xia / xian / xiang
        assert!(!SYLLABLE_TRIE.matches("xiangmu").whole_is_prefix);
    }

    #[test]
    fn proper_prefix_excludes_complete_syllables() {
        assert!(SYLLABLE_TRIE.is_proper_prefix("zh"));
        assert!(SYLLABLE_TRIE.is_proper_prefix("xia"));
        assert!(!SYLLABLE_TRIE.is_proper_prefix("zhuang"));
        assert!(!SYLLABLE_TRIE.is_proper_prefix("v"));
        assert!(!SYLLABLE_TRIE.is_proper_prefix(""));
    }

    #[test]
    fn agrees_with_linear_scan_on_every_prefix() {
        for syllable in SYLLABLES {
            for end in 1..=syllable.len() {
                let s = &syllable[..end];
                let linear_prefix = SYLLABLES
                    .iter()
                    .any(|t| t.len() > s.len() && t.starts_with(s));
                assert_eq!(SYLLABLE_TRIE.is_proper_prefix(s), linear_prefix, "{s}");
                assert_eq!(SYLLABLE_TRIE.contains(s), SYLLABLES.contains(&s), "{s}");
            }
        }
    }
}
