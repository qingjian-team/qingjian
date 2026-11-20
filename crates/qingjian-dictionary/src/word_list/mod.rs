//! 词表（英文、越南语……）：按小写编码查原样大小写的词，也能按前缀补全。
//!
//! 9.5 万词全常驻，所以不给每个词单独分配：编码与原样词拼进一块文字缓冲，条目只记位置（[`Entry`]），按编码排序后二分查。
//! 打包成 `.qj` 时这两块原样落盘，打开时直接映射，不用再解析、排序。
//!
//! 音标表（`english-ipa.tsv`：`词\t/音标/`，随包打成 `english-ipa.qj`）用同一套结构加载——两个字符列、按码排序，
//! 只是「原样词」那列放的是 `/ˈkɑrɡoʊ/` 这样的音标，用 [`WordList::pronunciation`] 取。
//! 查询一律按小写码，与英文候选查词走同一条路径。

mod entry;
mod load;
mod query;

#[cfg(test)]
mod tests;

use qingjian_format::{Table, Text};

use entry::Entry;

/// `.qj` 里的分节：文字缓冲、按编码排好的条目表。
const TEXT_TAG: [u8; 4] = *b"TEXT";
const ENTRIES_TAG: [u8; 4] = *b"ENTR";

/// 文件格式 TSV：`词\t编码[\t词频]`（编码缺省为词的小写，词频缺省 0），或 `pack wordlist` 打的 `.qj`。
#[derive(Debug, Default)]
pub struct WordList {
    /// 所有编码与原样词拼在一起。
    text: Text,

    /// 按编码字节序排好：精确查与前缀补全都二分定位。
    entries: Table<Entry>,
}

impl WordList {
    fn code(&self, entry: &Entry) -> &str {
        let start = entry.code_start as usize;
        &self.text[start..start + usize::from(entry.code_len)]
    }

    fn word(&self, entry: &Entry) -> &str {
        let start = entry.word_start as usize;
        &self.text[start..start + usize::from(entry.word_len)]
    }

    fn find(&self, code: &str) -> Option<&Entry> {
        self.entries
            .binary_search_by(|entry| self.code(entry).cmp(code))
            .ok()
            .map(|index| &self.entries[index])
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
