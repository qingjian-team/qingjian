//! 词库仓库的清单 `index.json`，缓存成 `dict-index.json`；怎么取见 [`crate::jobs::remote_index`]。

mod author;
mod dict_kind;
mod entry;
mod index_file;
mod shelf;

pub use author::{Author, is_safe_link};
pub use dict_kind::DictKind;
pub use entry::DictEntry;
pub use shelf::Shelf;

use crate::jobs::download::Repo;
use crate::jobs::remote_index::{Listing, RemoteIndex};

pub type DictIndex = RemoteIndex<DictEntry>;

pub type IndexState = crate::jobs::remote_index::IndexState<DictEntry>;

pub use crate::jobs::remote_index::Status;

impl Listing for DictEntry {
    const REPO: Repo = Repo::Dicts;

    const CACHE_FILE: &'static str = "dict-index.json";

    const WHAT: &'static str = "dictionaries";

    fn parse(bytes: &[u8]) -> Result<Vec<Self>, String> {
        index_file::IndexFile::parse(bytes)
    }

    fn id(&self) -> &str {
        &self.id
    }
}
