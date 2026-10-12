//! 主题预览图：锁定外观的主题出两张，没锁定的多一张深色竖排；PNG 写在指定目录。

use std::path::{Path, PathBuf};

use qingjian_render::FontLibrary;
use qingjian_render::preview::write_previews;

fn out_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-preview-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn written(theme: &str) -> Vec<String> {
    let theme_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("themes")
        .join(theme);
    let out = out_dir(theme);
    let shots = write_previews(&theme_dir, &out, |_| Vec::new()).expect("主题读得进来");
    let files = shots
        .iter()
        .map(|shot| {
            let path = shot.result.as_ref().expect("每张都画得出来");
            assert!(path.is_file() && path.starts_with(&out));
            shot.file.to_owned()
        })
        .collect();
    std::fs::remove_dir_all(&out).unwrap();
    files
}

#[test]
fn locked_theme_gets_vertical_and_horizontal() {
    if FontLibrary::system("zh-CN").is_err() {
        eprintln!("没有系统字体，跳过");
        return;
    }
    assert_eq!(written("sakura"), ["vertical.png", "horizontal.png"]);
}

#[test]
fn theme_with_both_appearances_also_gets_dark_vertical() {
    if FontLibrary::system("zh-CN").is_err() {
        eprintln!("没有系统字体，跳过");
        return;
    }
    assert_eq!(
        written("qingjian"),
        ["vertical.png", "horizontal.png", "vertical-dark.png"]
    );
}

#[test]
fn missing_theme_fails_as_a_whole() {
    let out = out_dir("missing");
    assert!(write_previews(Path::new("/nonexistent/theme"), &out, |_| Vec::new()).is_err());
    assert!(!out.exists(), "主题读不进来时不建输出目录");
}
