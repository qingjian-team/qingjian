//! 本机装的字族名，给字体下拉框列出来。按名字表读，不加载字形；第一次打开下拉框时才扫。

/// 字族名，排好序、去掉重复与系统私用的（`.` 开头）。
pub fn families() -> Vec<String> {
    let mut names = scan();
    names.retain(|name| !name.starts_with('.'));
    names.sort();
    names.dedup();
    names
}

#[cfg(windows)]
fn scan() -> Vec<String> {
    bamboo_slip::system_fonts::families()
}

#[cfg(not(windows))]
fn scan() -> Vec<String> {
    let mut db = cosmic_text::fontdb::Database::new();
    for dir in directories() {
        db.load_fonts_dir(dir);
    }
    db.faces()
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        .collect()
}

#[cfg(not(windows))]
fn directories() -> Vec<std::path::PathBuf> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let mut dirs: Vec<std::path::PathBuf> = if cfg!(target_os = "macos") {
        ["/System/Library/Fonts", "/Library/Fonts"]
            .iter()
            .map(std::path::PathBuf::from)
            .collect()
    } else {
        ["/usr/share/fonts", "/usr/local/share/fonts"]
            .iter()
            .map(std::path::PathBuf::from)
            .collect()
    };
    if let Some(home) = home {
        if cfg!(target_os = "macos") {
            dirs.push(home.join("Library/Fonts"));
        } else {
            dirs.push(home.join(".local/share/fonts"));
            dirs.push(home.join(".fonts"));
        }
    }
    dirs
}
