//! 清单里的一本词库，字段由词库仓库的 `tools/index.py` 按各本 `dict.toml` 与打好的 `.qjz` 写出。

use qingjian_format::{FORMAT_VERSION, Kind};
use serde::Deserialize;

use crate::jobs::download::{Install, Item, Repo, Target};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DictEntry {
    pub id: String,

    pub name: String,

    /// `base` / `extra` / `glossary` / `english`；现在能下载的只有附加词库 `extra`。
    pub kind: String,

    pub version: String,

    /// Release 标签，`<id>-v<版本>`。
    pub tag: String,

    /// `.qjz` 的大小。
    pub bytes: u64,

    /// 解出来的 `.qj` 的格式版本。
    pub qj_format: u16,

    pub license: String,

    #[serde(default)]
    pub description: String,

    /// 是不是青简维护的（`official/`），第三方的是 `false`。
    #[serde(default)]
    pub official: bool,
}

impl DictEntry {
    /// 这一版输入法能不能显示它（附加词库才在「词库」页列出）。
    pub fn listed(&self) -> bool {
        self.kind == "extra" && self.qj_format >= FORMAT_VERSION
    }

    /// 格式比这一版输入法新：列出来但不能下，提示先更新输入法。
    pub fn needs_newer_app(&self) -> bool {
        self.qj_format > FORMAT_VERSION
    }

    /// 装好后在用户词库目录里的文件名主干，与导入的词库同一套开关。
    pub fn stem(&self) -> &str {
        &self.id
    }

    pub fn target(&self) -> Target {
        Target {
            item: Item::Dict(self.id.clone()),
            repo: Repo::Dicts,
            tag: self.tag.clone(),
            install: Install::Qjz {
                dest: format!("dicts/{}.qj", self.id),
                kind: Kind::Dictionary,
            },
        }
    }
}
