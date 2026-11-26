//! 模型仓库的清单 `index.json`，缓存成 `model-index.json`；怎么取见 [`crate::jobs::remote_index`]。
//! 从没取到过、也没有缓存时（第一次打开就没网）用随程序带的 `builtin.json`：模型仓库清单的一份拷贝，装过的模型照样列得出来、能开关。

mod entry;
mod index_file;

pub use entry::{Engine, ModelEntry};

use crate::jobs::download::Repo;
use crate::jobs::remote_index::{Listing, RemoteIndex};

pub type ModelIndex = RemoteIndex<ModelEntry>;

impl Listing for ModelEntry {
    const REPO: Repo = Repo::Models;

    const CACHE_FILE: &'static str = "model-index.json";

    const WHAT: &'static str = "models";

    fn parse(bytes: &[u8]) -> Result<Vec<Self>, String> {
        index_file::IndexFile::parse(bytes)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn fallback() -> Vec<Self> {
        Self::parse(include_bytes!("builtin.json")).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_copy_lists_every_model_the_page_offers() {
        let ids: Vec<String> = ModelEntry::fallback()
            .into_iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(ids, ["zhiyi", "zhiyi-feather", "hanzhang-tongbian"]);
    }
}
