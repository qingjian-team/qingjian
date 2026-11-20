//! 内置主题与文字样式的回归测试（从原 `mod.rs` 内联测试原样迁出，断言未改）。

use bamboo_slip::{Color, FontWeight};

use super::builtins::BUILTINS;
use super::{TextSizes, Theme};
use crate::theme::assets;
use crate::theme::file::{ColorRef, FontRef};
use crate::theme::validate;

#[test]
fn builtin_themes_parse_without_problems() {
    for theme in Theme::builtins() {
        assert!(
            validate::problems(theme.file()).is_empty(),
            "{}: {:?}",
            theme.id(),
            validate::problems(theme.file())
        );
    }
    assert_eq!(Theme::builtins().len(), BUILTINS.len());
    let theme = Theme::light();
    assert_eq!(theme.id(), "qingjian");
    assert!(Theme::builtin("wechat", true).is_some());
    assert!(Theme::builtin("nope", false).is_none());
    assert_eq!(
        theme.color(&ColorRef::Variable("accent".to_owned())),
        Color::rgba(148, 190, 82, 127)
    );
    assert_eq!(
        Theme::dark().color(&ColorRef::Variable("accent".to_owned())),
        Color::rgb(51, 111, 51)
    );
}

#[test]
fn theme_file_accepts_comments_trailing_commas_and_schema_key() {
    let theme = Theme::from_json(
        r##"{
            "$schema": "https://qingjian.app/schema/theme-1.json",
            "extends": "qingjian", // 以青简绿为底
            "schema": 1,
            "meta": { "id": "jsonc", "name": "注释", },
            /* 只改高亮色 */
            "variables": { "accent": "#ffb7d5", },
        }"##,
        false,
    )
    .unwrap();
    assert_eq!(theme.id(), "jsonc");
    assert_eq!(
        theme.color(&ColorRef::Variable("accent".to_owned())),
        Color::rgb(0xff, 0xb7, 0xd5)
    );
}

#[test]
fn builtin_images_are_all_embedded() {
    for theme in Theme::builtins() {
        for path in assets::image_paths(theme.file()) {
            assert!(
                theme.image(&path).is_some() || theme.svg(&path).is_some(),
                "内置主题 {} 用到的 {path} 没有编进程序",
                theme.id()
            );
        }
    }
}

#[test]
fn user_theme_inherits_builtin_images() {
    let dir = std::env::temp_dir().join("qingjian-theme-inherits-sakura");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("theme.json"),
        r##"{ "extends": "sakura", "schema": 1, "meta": { "id": "my-sakura", "name": "我的樱花" },
             "variables": { "text": "#000000" } }"##,
    )
    .unwrap();
    let theme = Theme::from_dir(&dir, false).unwrap();
    assert!(theme.image("images/character.png").is_some());
    assert!(theme.svg("images/heart.svg").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn user_theme_without_extends_builds_on_qingjian() {
    let theme = Theme::from_json(
        r##"{ "schema": 1, "meta": { "id": "red", "name": "红" }, "variables": { "accent": "#ff0000" } }"##,
        false,
    )
    .unwrap();
    assert_eq!(
        theme.color(&ColorRef::Variable("accent".to_owned())),
        Color::rgb(255, 0, 0)
    );
    assert_eq!(theme.font("candidate"), Theme::light().font("candidate"));
}

#[test]
fn text_sizes_scale_styles_and_rows() {
    let theme = Theme::light();
    let (candidate, index, annotation) = (
        theme.font("candidate"),
        theme.font("index"),
        theme.font("annotation"),
    );
    let ratio = 26.0 / candidate.size;
    let sized = theme.with_text_sizes(TextSizes::new(26.0, 0.0));
    assert_eq!(sized.font("candidate").size, 26.0);
    assert_eq!(
        sized.font("candidate").line_height,
        candidate.line_height * ratio
    );
    assert_eq!(sized.font("index").size, index.size * ratio);
    assert_eq!(sized.font("annotation"), annotation);
    assert_eq!(sized.candidate_scale(), ratio);
    // 换外观不丢设置里的字号
    assert_eq!(sized.with_dark(true).font("candidate").size, 26.0);
    // 在样式上写死的字号（手机候选条）按同一比例缩放
    let inline: FontRef =
        serde_json::from_str(r#"{ "base": "candidate", "size": 19, "line_height": 23 }"#)
            .unwrap();
    assert_eq!(theme.font_ref(&inline).size, 19.0);
    assert_eq!(sized.font_ref(&inline).size, 19.0 * ratio);
    assert_eq!(sized.font_ref(&inline).line_height, 23.0 * ratio);
    // 不是正数当没填
    assert_eq!(TextSizes::new(-1.0, f32::NAN), TextSizes::default());
}

#[test]
fn candidate_weight_overrides_the_candidate_style_only() {
    let theme = Theme::light();
    let bold = theme.with_candidate_weight(Some(700));
    assert_eq!(bold.font("candidate").weight, FontWeight(700));
    assert_eq!(bold.font("annotation"), theme.font("annotation"));
    assert_eq!(
        bold.with_dark(true).font("candidate").weight,
        FontWeight(700)
    );
    assert_eq!(
        bold.theme_weight("candidate"),
        theme.theme_weight("candidate")
    );
    // 节点上自己写了字重的不动，只写字号的跟着设置
    let semibold: FontRef =
        serde_json::from_str(r#"{ "base": "candidate", "weight": "semibold" }"#).unwrap();
    assert_eq!(bold.font_ref(&semibold).weight, FontWeight(600));
    let sized: FontRef =
        serde_json::from_str(r#"{ "base": "candidate", "size": 19 }"#).unwrap();
    assert_eq!(bold.font_ref(&sized).weight, FontWeight(700));
    assert_eq!(
        theme.with_candidate_weight(None).font("candidate"),
        theme.font("candidate")
    );
}
