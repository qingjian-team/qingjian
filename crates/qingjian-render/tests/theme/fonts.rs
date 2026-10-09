//! 主题字体：样式写的系统字族由壳查文件交给渲染器加载，随主题带的字体从主题目录加载；回退链挑第一个装了的。
//! 用系统自带的 Georgia 当样例，系统里没有时跳过。

use std::path::{Path, PathBuf};

use qingjian_render::{FontLibrary, Renderer, Theme};

/// 这个平台上 Georgia 的文件。
fn georgia() -> Option<PathBuf> {
    [
        "/System/Library/Fonts/Supplemental/Georgia.ttf",
        r"C:\Windows\Fonts\georgia.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.is_file())
}

fn renderer() -> Option<Renderer> {
    FontLibrary::system("zh-CN").ok().map(Renderer::new)
}

fn theme_json(candidate_family: &str, fonts: &str) -> String {
    format!(
        r#"{{ "extends": "qingjian", "schema": 1, "meta": {{ "id": "fonts", "name": "fonts" }}, "fonts": {fonts},
             "text": {{ "styles": {{ "candidate": {{ "size": 16, "line_height": 19, "family": {candidate_family} }} }} }} }}"#
    )
}

#[test]
fn system_family_is_loaded_through_the_shell_lookup() {
    let (Some(file), Some(mut renderer)) = (georgia(), renderer()) else {
        eprintln!("没有 Georgia 或系统字体，跳过");
        return;
    };
    let theme = Theme::from_json(
        &theme_json(r#"["No Such Font", "Georgia", "system"]"#, "[]"),
        false,
    )
    .unwrap();
    assert_eq!(theme.font_families(), ["No Such Font", "Georgia"]);
    let before = renderer.trace_families("Abc", &theme);
    assert!(!before.contains(&"Georgia".to_owned()), "{before:?}");

    let mut asked = Vec::new();
    let lookup = |family: &str| {
        if family == "Georgia" {
            vec![file.clone()]
        } else {
            Vec::new()
        }
    };
    renderer.load_theme_fonts(&theme, |family| {
        asked.push(family.to_owned());
        lookup(family)
    });
    assert_eq!(asked, ["No Such Font", "Georgia"]);
    assert_eq!(renderer.trace_families("Abc", &theme), ["Georgia"]);
}

#[test]
fn missing_families_fall_back_to_the_ui_font() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let plain = Theme::light();
    let theme = Theme::from_json(&theme_json(r#""No Such Font""#, "[]"), false).unwrap();
    renderer.load_theme_fonts(&theme, |_| Vec::new());
    assert_eq!(
        renderer.trace_families("Abc", &theme),
        renderer.trace_families("Abc", &plain)
    );
}

#[test]
fn bundled_font_loads_from_the_theme_directory() {
    let (Some(file), Some(mut renderer)) = (georgia(), renderer()) else {
        eprintln!("没有 Georgia 或系统字体，跳过");
        return;
    };
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("theme-fonts");
    std::fs::create_dir_all(dir.join("fonts")).unwrap();
    std::fs::copy(&file, dir.join("fonts/bundled.ttf")).unwrap();
    let json = theme_json(
        r#""Georgia""#,
        r#"[{ "file": "fonts/bundled.ttf" }, { "file": "../outside.ttf" }, { "file": "fonts/missing.ttf" }]"#,
    );
    std::fs::write(dir.join("theme.json"), json).unwrap();

    let theme = Theme::from_dir(&dir, false).unwrap();
    assert_eq!(theme.font_files(), [dir.join("fonts/bundled.ttf")]);
    // 随包字体先加载，字体库里有了 Georgia 就不再向壳查
    renderer.load_theme_fonts(&theme, |family| panic!("不该查 {family}"));
    assert_eq!(renderer.trace_families("Abc", &theme), ["Georgia"]);
    assert!(Path::new(&theme.font_files()[0]).starts_with(&dir));
}
