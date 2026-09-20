//! 辅码码表：词 → 码的映射（`AuxCodeTable` 的 `.qj`），辅码态逐键即筛用；与五笔那类编码 → 词的形码码表方向相反。
//!
//! 不复用词库存储的理由见 `docs/design/aux-code.md`。
//! 分节：`TEXT` 词文本 arena、`CODES` 码键 arena、`ENTR` 条目表（按词文本字节序升序，同词的条目相邻）、
//! `HASH` 词 → 条目区间的哈希索引（槽 = 区间起点，键 = 词文本）。

mod columns;
mod entry;
mod import;
mod imported;
mod info;
mod lookup;
mod parsed;
mod report;
mod table;

#[cfg(test)]
mod tests;

pub use import::import_aux_code_table;
pub use imported::AuxCodeTableImport;
pub use info::{AuxCodeTableInfo, aux_code_table_info};
pub use lookup::AuxCodeLookup;
pub use report::AuxCodeTableImportReport;
pub use table::AuxCodeTable;
