//! Linux 的字体文件清单：发行版装 Noto 的常见位置。以后读 fontconfig 配置再补。

use std::path::PathBuf;

pub(super) fn ui_fonts() -> Vec<PathBuf> {
    [
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

pub(super) fn ui_weight_fonts() -> Vec<PathBuf> {
    [
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/noto/NotoSans-Bold.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

pub(super) fn script_fonts(_locale: &str) -> Vec<PathBuf> {
    [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

pub(super) fn emoji_fonts() -> Vec<PathBuf> {
    [
        "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
        "/usr/share/fonts/noto/NotoColorEmoji.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}
