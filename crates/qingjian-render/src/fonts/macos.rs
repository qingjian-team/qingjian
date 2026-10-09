//! macOS 的字体文件清单。系统字体 SF 与 PingFang 不在 `/Library/Fonts`，路径是系统私有位置，版本升级可能挪，所以每个角色给几个候选。

use std::path::PathBuf;

const SYSTEM_FONTS: &str = "/System/Library/Fonts";

/// 界面字体：SF Pro（文件名 SFNS，字族名 `.SF NS`）。
pub(super) fn ui_fonts() -> Vec<PathBuf> {
    vec![
        PathBuf::from(format!("{SYSTEM_FONTS}/SFNS.ttf")),
        PathBuf::from(format!("{SYSTEM_FONTS}/Helvetica.ttc")),
    ]
}

/// 界面字体的其余字重：SF 是可变字体、苹方是多字重集合，不用另加文件。
pub(super) fn ui_weight_fonts() -> Vec<PathBuf> {
    Vec::new()
}

/// 按 locale 要的汉字字体，加日文作为日语译文的回退。
pub(super) fn script_fonts(locale: &str) -> Vec<PathBuf> {
    let mut fonts = Vec::new();
    if !locale.starts_with("ja") {
        fonts.extend(pingfang_paths());
        // 没找到 PingFang 时的简体兜底
        fonts.push(PathBuf::from(format!(
            "{SYSTEM_FONTS}/Hiragino Sans GB.ttc"
        )));
    }
    fonts.push(PathBuf::from(format!(
        "{SYSTEM_FONTS}/ヒラギノ角ゴシック W4.ttc"
    )));
    fonts
}

pub(super) fn emoji_fonts() -> Vec<PathBuf> {
    vec![PathBuf::from(format!(
        "{SYSTEM_FONTS}/Apple Color Emoji.ttc"
    ))]
}

/// PingFang.ttc 在系统资产目录里，目录名带哈希，要扫一层找出来；找不到退回 FontServices 里的 UI 版。
fn pingfang_paths() -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(assets) = std::fs::read_dir("/System/Library/AssetsV2") {
        for bucket in assets.flatten() {
            if !bucket
                .file_name()
                .to_string_lossy()
                .starts_with("com_apple_MobileAsset_Font")
            {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(bucket.path()) else {
                continue;
            };
            for asset in entries.flatten() {
                let candidate = asset.path().join("AssetData/PingFang.ttc");
                if candidate.is_file() {
                    found.push(candidate);
                }
            }
        }
    }
    found.push(PathBuf::from(
        "/System/Library/PrivateFrameworks/FontServices.framework/Versions/A/Resources/Reserved/PingFangUI.ttc",
    ));
    found
}
