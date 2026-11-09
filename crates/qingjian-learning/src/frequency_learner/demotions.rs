//! 降权表：用户在候选菜单里对某个输入串下的某个词点「降权」，之后这个输入串下它往后挪几格。

use super::*;

impl FrequencyLearner {
    /// 降权表与词频文件同目录。
    pub(super) fn demoted_path(frequency_path: &Path) -> PathBuf {
        frequency_path.with_file_name(USER_DEMOTED_FILE)
    }

    /// 从 `输入串\t词\t次数` 读降权表，返回跳过的坏行数。
    pub(super) fn load_demoted(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let Some((input, text, count)) = parse_counted_pair(line) else {
                skipped += 1;
                continue;
            };
            if count > 0 {
                self.demoted
                    .entry(input.to_owned())
                    .or_default()
                    .insert(text.to_owned(), count.min(MAX_DEMOTION));
            }
        }
        skipped
    }

    pub(super) fn save_demoted_to(&mut self, path: &Path) -> Result<(), LearningError> {
        let mut rows: Vec<(&String, &String, &u32)> = self
            .demoted
            .iter()
            .flat_map(|(input, texts)| texts.iter().map(move |(text, count)| (input, text, count)))
            .collect();
        rows.sort();
        write_atomic(path, |file| {
            writeln!(file, "# 青简候选降权：输入串\t词\t降了几次")?;
            for (input, text, count) in rows {
                writeln!(file, "{input}\t{text}\t{count}")?;
            }
            Ok(())
        })?;
        self.demoted_dirty = false;
        Ok(())
    }

    /// 降权表里有几条 (输入串, 词)。
    pub fn demoted_count(&self) -> usize {
        self.demoted.values().map(HashMap::len).sum()
    }
}
