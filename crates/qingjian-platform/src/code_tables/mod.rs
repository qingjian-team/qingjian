//! 辅码码表：随包的（随包根 `codes/`）与用户目录 `codes/` 下的 `.qj`，连同 `schemes/` 里的辅码方案，
//! 按 `[aux_code] scheme` 挑一套接到 Engine 上。

mod source;
mod sources;

pub use source::AuxSource;
pub use sources::AuxCodeSources;

use std::path::{Path, PathBuf};

use qingjian_dictionary::AuxWordRules;

/// 随包的笔画码表（`codes/stroke.qj`）。
pub const STROKE: &str = "stroke";

/// 码表的扩展名：只认打包过的 `.qj`（TSV 在导入时就转掉了）。
const EXTENSION: &str = "qj";

/// 笔画的词码规则，与 `dict-convert` 打包时给词库里的词算码同一条（每字首笔，四字以上取前三字与末字）：
/// 表里只收了基础词库的词，自造词、导入词库与学到的词靠它现算，不然进辅码就被藏起来。
fn stroke_word_rules() -> AuxWordRules {
    AuxWordRules::parse([(2, "AaBa"), (3, "AaBaCa"), (4, "AaBaCaZa")]).unwrap_or_default()
}

/// 列出目录里的码表文件（按文件名排序），返回 (文件名不含扩展名, 路径)。目录不存在就是空。
pub fn list(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter_map(|path| {
            (path.extension()?.to_str()? == EXTENSION).then(|| {
                let stem = path.file_stem()?.to_str()?.to_owned();
                Some((stem, path))
            })?
        })
        .collect();
    files.sort();
    files
}
