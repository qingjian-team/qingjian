//! Windows 的字体文件清单：Segoe UI、微软雅黑、Yu Gothic、Segoe UI Emoji（COLRv0）。

use std::path::PathBuf;

fn fonts_dir() -> PathBuf {
    std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("Fonts")
}

pub(super) fn ui_fonts() -> Vec<PathBuf> {
    vec![
        fonts_dir().join("segoeui.ttf"),
        fonts_dir().join("arial.ttf"),
    ]
}

/// Segoe UI 的其余字重（主题里写 `weight` 时用）：细、半细、半粗、粗、特粗各一个文件。
pub(super) fn ui_weight_fonts() -> Vec<PathBuf> {
    let dir = fonts_dir();
    [
        "segoeuil.ttf",
        "segoeuisl.ttf",
        "seguisb.ttf",
        "segoeuib.ttf",
        "seguibl.ttf",
    ]
    .into_iter()
    .map(|file| dir.join(file))
    .collect()
}

/// 汉字字体（常规、粗、细三个文件）加日文回退。
pub(super) fn script_fonts(locale: &str) -> Vec<PathBuf> {
    let dir = fonts_dir();
    let mut fonts = Vec::new();
    if locale.starts_with("zh-TW") || locale.starts_with("zh-HK") {
        fonts.extend(["msjh.ttc", "msjhbd.ttc", "msjhl.ttc"].map(|file| dir.join(file)));
    } else if !locale.starts_with("ja") {
        fonts.extend(["msyh.ttc", "msyhbd.ttc", "msyhl.ttc"].map(|file| dir.join(file)));
    }
    fonts.push(dir.join("YuGothR.ttc"));
    fonts.push(dir.join("yugothm.ttc"));
    fonts.push(dir.join("YuGothB.ttc"));
    fonts.push(dir.join("YuGothL.ttc"));
    fonts
}

pub(super) fn emoji_fonts() -> Vec<PathBuf> {
    vec![fonts_dir().join("seguiemj.ttf")]
}
