//! Linux 形码（五笔）码表装配。
use std::path::Path;

use qingjian_core::Engine;
use qingjian_dictionary::CodeTable;
use qingjian_platform::Scheme;

/// 按配置装配拼音与五笔两条输入轴。
pub fn apply_code_table(engine: &mut Engine, pinyin: Scheme, wubi: bool, path: Option<&Path>) {
    engine.set_shuangpin(pinyin.shuangpin());
    engine.set_zhuyin_mode(pinyin == Scheme::Zhuyin);
    engine.set_phonetic(pinyin.is_on() || !wubi);
    if !wubi {
        engine.set_code_table(None);
        return;
    }
    let Some(path) = path else {
        tracing::warn!("已启用五笔，但找不到 assets/wubi/wubi86.tsv，按拼音输入");
        engine.set_code_table(None);
        return;
    };
    match CodeTable::from_path(path) {
        Ok(table) => {
            tracing::info!(table = %path.display(), entries = table.len(), "Linux 五笔码表已载入");
            engine.set_code_table(Some(table));
        }
        Err(error) => {
            tracing::error!(%error, table = %path.display(), "Linux 五笔码表读取失败，按拼音输入");
            engine.set_code_table(None);
        }
    }
}
