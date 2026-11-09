//! 导出成 TSV：`词\t拼音\t词频`，与导入、`Dictionary::parse` 读的同一种格式，导出再导入得到同一本词库。

use std::io::{self, Write};

use super::Dictionary;

impl Dictionary {
    /// 全部词目写成 TSV，返回写了几条。开头几行 `#` 注释写名称、许可与署名（有元数据时），许可要求署名的词库导出后也带着出处。
    /// 拼音音节之间一个空格，知道声调的音节带数字调（`ni3 hao3`），不知道的不带。
    pub fn write_tsv(&self, out: &mut impl Write) -> io::Result<usize> {
        if let Some(metadata) = self.metadata() {
            for (label, value) in [
                ("name", &metadata.name),
                ("version", &metadata.version),
                ("license", &metadata.license),
                ("attribution", &metadata.attribution),
                ("source", &metadata.source),
            ] {
                if !value.is_empty() {
                    writeln!(out, "# {label}: {}", value.replace('\n', " "))?;
                }
            }
        }
        let mut count = 0;
        let mut pinyin = String::new();
        for entry in self.entries() {
            pinyin.clear();
            for (index, syllable) in entry.pinyin.split(' ').enumerate() {
                if index > 0 {
                    pinyin.push(' ');
                }
                pinyin.push_str(syllable);
                let tone = entry.tones.get(index);
                if tone != 0 {
                    pinyin.push(char::from(b'0' + tone));
                }
            }
            writeln!(out, "{}\t{pinyin}\t{}", entry.text, entry.frequency)?;
            count += 1;
        }
        Ok(count)
    }
}
