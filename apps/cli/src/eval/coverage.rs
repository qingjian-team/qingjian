//! 码表覆盖率：词库里的词有多少能查到码。
//!
//! 与导入统计同源：都用码表的 `AuxCodeLookup`，两段口径分别是「词频前 10,000」与「全库」。

use qingjian_core::Engine;

/// 覆盖率只看词频最高的这么多条（缺省口径）。
pub const COVERAGE_TOP: usize = 10_000;

/// 两段覆盖率的分子分母。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    /// 词频前 `COVERAGE_TOP` 条里能查到码的条数 / 总数。
    pub top_hits: usize,
    pub top_total: usize,

    /// 全库能查到码的条数 / 总数。
    pub all_hits: usize,
    pub all_total: usize,
}

/// 算一遍覆盖率；没装码表时返回 `None`（这条尺子只在 `--aux-table` 下才有意义）。
pub fn measure(engine: &Engine) -> Option<Coverage> {
    let tables = engine.aux_codes();
    if tables.is_empty() {
        return None;
    }
    let covers = |text: &str| {
        tables
            .iter()
            .any(|table| table.code_with_prefix(text, "").is_some())
    };
    let mut words: Vec<(&str, u32)> = engine
        .dictionary()
        .entries()
        .map(|hit| (hit.text, hit.frequency))
        .collect();
    let all_total = words.len();
    let all_hits = words.iter().filter(|(text, _)| covers(text)).count();
    words.sort_by_key(|(_, frequency)| std::cmp::Reverse(*frequency));
    words.truncate(COVERAGE_TOP);
    let top_total = words.len();
    let top_hits = words.iter().filter(|(text, _)| covers(text)).count();
    Some(Coverage {
        top_hits,
        top_total,
        all_hits,
        all_total,
    })
}
