//! 清单里的一本词库，字段由词库仓库的 `tools/index.py` 按各本 `dict.toml` 与打好的 `.qjz` 写出。

use qingjian_core::Language;
use qingjian_format::{FORMAT_VERSION, Kind};
use qingjian_translate::DOWNLOADED_DIR;
use serde::Deserialize;

use super::{Author, DictKind};
use crate::jobs::download::{Install, Item, Repo, Target};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DictEntry {
    pub id: String,

    pub name: String,

    /// 能下载的是附加词库与释义表。
    pub kind: DictKind,

    /// 释义表的语言代码（`ja`），别的种类没有。
    #[serde(default)]
    pub language: Option<String>,

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

    /// 做这本的人或团队；上游与外部贡献者的署名在 `.qj` 元数据里，这里不带。
    #[serde(default)]
    pub author: Author,

    /// 是不是青简维护的（`official/`），第三方的是 `false`。
    #[serde(default)]
    pub official: bool,
}

impl DictEntry {
    /// 这一版输入法能不能显示它：附加词库，或语言认得的释义表。
    pub fn listed(&self) -> bool {
        let known = self.kind == DictKind::Extra || self.glossary_language().is_some();
        known && self.qj_format >= FORMAT_VERSION
    }

    /// 格式比这一版输入法新：列出来但不能下，提示先更新输入法。
    pub fn needs_newer_app(&self) -> bool {
        self.qj_format > FORMAT_VERSION
    }

    /// 释义表的语言；不是释义表或语言不认得为 `None`。
    pub fn glossary_language(&self) -> Option<Language> {
        if self.kind != DictKind::Glossary {
            return None;
        }
        self.language.as_deref()?.parse().ok()
    }

    /// 装好后的文件名主干：附加词库在 `downloads/dicts/` 里与导入的同一套开关，释义表在 `downloads/glossaries/` 里是 `glossary-<语言>`。
    pub fn stem(&self) -> String {
        match self.glossary_language() {
            Some(language) => format!("glossary-{}", language.code()),
            None => self.id.clone(),
        }
    }

    pub fn target(&self) -> Target {
        let (dest, kind) = match self.glossary_language() {
            Some(_) => (
                format!("{DOWNLOADED_DIR}/{}.qj", self.stem()),
                Kind::Glossary,
            ),
            None => (format!("dicts/{}.qj", self.id), Kind::Dictionary),
        };
        Target {
            item: Item::Dict(self.id.clone()),
            repo: Repo::Dicts,
            tag: self.tag.clone(),
            install: Install::Qjz { dest, kind },
        }
    }
}
