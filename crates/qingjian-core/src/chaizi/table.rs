//! 拆字表的内存结构：部件拼音串 → 字（按字频的行号）。

use std::collections::BTreeMap;

pub struct ChaiziTable {
    /// 按字频从高到低：(字, 最常见的读音)。
    entries: Vec<(char, &'static str)>,

    /// 部件拼音串 → `entries` 的下标，升序（字频高的在前）。
    keys: BTreeMap<&'static str, Vec<u32>>,
}

impl ChaiziTable {
    /// 解析 `字\t读音…\t部件拼音串…`，`#` 开头的是注释。
    pub fn parse(source: &'static str) -> Self {
        let mut entries = Vec::new();
        let mut keys: BTreeMap<&'static str, Vec<u32>> = BTreeMap::new();
        for line in source.lines().filter(|line| !line.starts_with('#')) {
            let mut fields = line.split('\t');
            let (Some(ch), Some(readings), Some(splits)) = (
                fields.next().and_then(|text| text.chars().next()),
                fields.next(),
                fields.next(),
            ) else {
                continue;
            };
            let index = entries.len() as u32;
            entries.push((ch, readings.split(' ').next().unwrap_or_default()));
            for key in splits.split(' ') {
                keys.entry(key).or_default().push(index);
            }
        }
        Self { entries, keys }
    }

    /// 整串对上的在前，其余以 `keys` 开头的在后，各自按字频；同一个字只出一次。`keys` 为空时没有结果。
    pub fn lookup(&self, keys: &str, limit: usize) -> Vec<(char, &'static str)> {
        if keys.is_empty() || limit == 0 {
            return Vec::new();
        }
        let exact = self.keys.get(keys).cloned().unwrap_or_default();
        let mut prefixed: Vec<u32> = self
            .keys
            .range::<str, _>((std::ops::Bound::Excluded(keys), std::ops::Bound::Unbounded))
            .take_while(|(key, _)| key.starts_with(keys))
            .flat_map(|(_, rows)| rows.iter().copied())
            .collect();
        prefixed.sort_unstable();
        prefixed.dedup();
        prefixed.retain(|row| !exact.contains(row));
        exact
            .into_iter()
            .chain(prefixed)
            .take(limit)
            .map(|row| self.entries[row as usize])
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::ChaiziTable;

    const SOURCE: &str = "# 注释\n林\tlin\tmumu\n森\tsen\tmumumu mulin\n沐\tmu\tshuimu\n";

    #[test]
    fn exact_keys_come_before_prefixes() {
        let table = ChaiziTable::parse(SOURCE);
        assert_eq!(table.lookup("mumu", 9), [('林', "lin"), ('森', "sen")]);
        assert_eq!(table.lookup("mumumu", 9), [('森', "sen")]);
        assert_eq!(table.lookup("mu", 9), [('林', "lin"), ('森', "sen")]);
        assert_eq!(table.lookup("shuimu", 9), [('沐', "mu")]);
        assert!(table.lookup("", 9).is_empty());
        assert!(table.lookup("xyz", 9).is_empty());
        assert_eq!(table.lookup("mu", 1).len(), 1);
    }
}
