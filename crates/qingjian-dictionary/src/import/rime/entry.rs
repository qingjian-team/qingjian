//! 导入阶段的一条词目；没有显式拼音的词留待字表注音。

#[derive(Debug)]
pub(super) struct Entry {
    pub word: String,

    pub code: String,

    pub weight: u32,
}

impl Entry {
    pub fn append_tsv(&self, output: &mut String) {
        use std::fmt::Write;

        writeln!(output, "{}\t{}\t{}", self.word, self.code, self.weight).unwrap();
    }
}
