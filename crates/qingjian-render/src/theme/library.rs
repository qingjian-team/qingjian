//! 主题库：内置主题加用户主题目录里的主题，设置界面列出的就是它。
//!
//! 用户主题放在 `<主题目录>/<id>/theme.json`（主题目录由壳给，在配置目录下的 `themes/`）。目录名必须等于 `meta.id`；
//! 不能与内置主题重名（想改内置主题就 `extends` 它、换个 id），外观词 `system` / `light` / `dark` 也不能当 id。
//! 读不进来的主题跳过并记警告，不影响其他主题。
//!
//! 热加载：壳在已有的每秒检查里调 [`ThemeLibrary::refresh`]，目录里 `theme.json` 的增删改（按修改时间与大小）会重读。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use super::Theme;

/// 用户主题的描述文件名。
const THEME_FILE: &str = "theme.json";

/// 外观的写法，不能当主题 id（旧配置里 `theme` 写的是它们）。
const RESERVED_IDS: [&str; 3] = ["system", "light", "dark"];

pub struct ThemeLibrary {
    /// 内置主题在前、用户主题按 id 排在后，都是浅色那一份。
    themes: Vec<Theme>,

    /// 用户主题目录；`None` 只有内置主题。
    dir: Option<PathBuf>,

    /// 上次读目录时的戳，见 [`Self::stamp`]。
    stamp: u64,
}

impl ThemeLibrary {
    /// 读内置主题与 `dir` 里的用户主题。目录不存在就只有内置主题。
    pub fn load(dir: Option<&Path>) -> Self {
        let stamp = dir.map_or(0, Self::stamp);
        let mut themes = Theme::builtins().to_vec();
        if let Some(dir) = dir {
            themes.extend(read_user_themes(dir));
        }
        Self {
            themes,
            dir: dir.map(Path::to_path_buf),
            stamp,
        }
    }

    /// 全部主题（浅色），内置在前。
    pub fn themes(&self) -> &[Theme] {
        &self.themes
    }

    /// 按 id 取主题并换成对应外观；没有这个 id 时退回缺省主题并记警告。
    pub fn resolve(&self, id: &str, dark: bool) -> Theme {
        match self.themes.iter().find(|theme| theme.id() == id) {
            Some(theme) => theme.with_dark(dark),
            None => {
                tracing::warn!(id, "没有这个主题，用缺省主题");
                Theme::light().with_dark(dark)
            }
        }
    }

    /// 用户主题目录有变化就重读，返回是否重读了。
    pub fn refresh(&mut self) -> bool {
        let Some(dir) = &self.dir else {
            return false;
        };
        let stamp = Self::stamp(dir);
        if stamp == self.stamp {
            return false;
        }
        *self = Self::load(Some(&dir.clone()));
        true
    }

    /// 目录戳：各主题子目录名与 `theme.json` 的修改时间、大小合成一个数，任何一个变了它就变。
    /// 只 `stat` 不读内容，每秒查一次的开销可以忽略。
    pub fn stamp(dir: &Path) -> u64 {
        let mut hasher = DefaultHasher::new();
        for (name, path) in theme_dirs(dir) {
            name.hash(&mut hasher);
            if let Ok(meta) = std::fs::metadata(path.join(THEME_FILE)) {
                meta.len().hash(&mut hasher);
                meta.modified().ok().hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

/// 主题目录下的子目录（名字、路径），按名字排序。
fn theme_dirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| Some((entry.file_name().into_string().ok()?, entry.path())))
        .collect();
    dirs.sort();
    dirs
}

/// 读用户主题，读不进来或不合规矩的跳过并记警告。
fn read_user_themes(dir: &Path) -> Vec<Theme> {
    let mut themes = Vec::new();
    for (name, path) in theme_dirs(dir) {
        let file = path.join(THEME_FILE);
        if !file.is_file() {
            continue;
        }
        let theme = match Theme::from_dir(&path, false) {
            Ok(theme) => theme,
            Err(error) => {
                tracing::warn!(path = %file.display(), %error, "用户主题读不进来，跳过");
                continue;
            }
        };
        let problem = if theme.id() != name {
            Some("meta.id 与目录名不一致")
        } else if RESERVED_IDS.contains(&theme.id()) {
            Some("外观词不能当主题 id")
        } else if Theme::builtin(theme.id(), false).is_some() {
            Some("与内置主题重名")
        } else {
            None
        };
        match problem {
            Some(problem) => {
                tracing::warn!(path = %file.display(), id = theme.id(), "用户主题{problem}，跳过");
            }
            None => themes.push(theme),
        }
    }
    themes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_theme(dir: &Path, name: &str, id: &str) {
        let path = dir.join(name);
        std::fs::create_dir_all(&path).unwrap();
        let json = format!(
            r##"{{ "extends": "qingjian", "meta": {{ "id": "{id}", "name": "{id} 主题" }},
                "variables": {{ "accent": "#ff0000" }} }}"##
        );
        std::fs::write(path.join(THEME_FILE), json).unwrap();
    }

    #[test]
    fn loads_user_themes_after_builtins_and_skips_bad_ones() {
        let dir = std::env::temp_dir().join("qingjian-theme-library-test");
        let _ = std::fs::remove_dir_all(&dir);
        write_theme(&dir, "peach", "peach");
        write_theme(&dir, "mismatch", "other");
        write_theme(&dir, "wechat", "wechat");
        write_theme(&dir, "dark", "dark");
        std::fs::create_dir_all(dir.join("broken")).unwrap();
        std::fs::write(dir.join("broken").join(THEME_FILE), "{").unwrap();

        let mut library = ThemeLibrary::load(Some(&dir));
        let ids: Vec<&str> = library.themes().iter().map(Theme::id).collect();
        let builtin = Theme::builtins().len();
        assert_eq!(ids.len(), builtin + 1);
        assert_eq!(ids[builtin], "peach");
        assert_eq!(library.resolve("peach", true).id(), "peach");
        assert_eq!(library.resolve("nope", false).id(), "qingjian");
        assert!(!library.refresh());

        write_theme(&dir, "maple", "maple");
        assert!(library.refresh());
        assert!(library.themes().iter().any(|theme| theme.id() == "maple"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
