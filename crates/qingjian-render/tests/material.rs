//! 系统材质：壳垫得了时窗口底色换成着色、不画投影，并报出材质与圆角；垫不了时照旧画主题的底色。

mod scenes;

use qingjian_render::{FontLibrary, Layout, Material, MaterialBacking, Renderer, Theme};

const THEME: &str = r##"{
  "extends": "qingjian",
  "schema": 1,
  "meta": { "id": "glass-test", "name": "测试", "author": "测试", "license": "MIT" },
  "variables": { "surface": "#ff0000" },
  "material": { "kind": "glass", "tint": "#00000000" },
  "windows": { "vertical": { "radius": 12 } }
}"##;

#[test]
fn material_replaces_the_window_fill_only_when_the_shell_supports_it() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let theme = Theme::from_json(THEME, false).unwrap();
    let frame = scenes::nihao();
    let mut renderer = Renderer::new(library);

    let opaque = renderer
        .render(&frame, Layout::Vertical, &theme, 2.0)
        .unwrap();
    assert_eq!(opaque.material, None);
    // 不透明窗口：投影留了边，内容区左上角往里一点是红底
    assert!(opaque.content_x > 0);
    let red = pixel(&opaque, opaque.content_x + 30, opaque.content_y + 4);
    assert_eq!((red[0], red[3]), (255, 255), "垫不了材质时照旧画 fill");

    renderer.set_material_support(true);
    let backed = renderer
        .render(&frame, Layout::Vertical, &theme, 2.0)
        .unwrap();
    assert_eq!(
        backed.material,
        Some(MaterialBacking {
            kind: Material::Glass,
            radius: 24.0,
            follows_backdrop: true,
            locked_dark: None,
        })
    );
    assert_eq!(
        (backed.content_x, backed.content_y),
        (0, 0),
        "投影交给系统，不留边"
    );
    assert_eq!(
        pixel(&backed, 30, 4)[3],
        0,
        "底色换成透明着色，材质从下面透出来"
    );
}

fn pixel(rendered: &qingjian_render::Rendered, x: u32, y: u32) -> [u8; 4] {
    let pixmap = &rendered.pixmap;
    let i = ((y * pixmap.width() + x) * 4) as usize;
    pixmap.data()[i..i + 4].try_into().unwrap()
}

/// 文字跟不跟着材质反色：`glass` 缺省跟，写 `system` 不跟，`blur` 缺省不跟；锁定了外观的主题只有一套颜色，写了 auto 也不跟。
#[test]
fn only_auto_follows_the_backdrop_and_a_locked_theme_never_does() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let mut renderer = Renderer::new(library);
    renderer.set_material_support(true);
    let frame = scenes::nihao();
    let backing = |renderer: &mut Renderer, json: &str| {
        let theme = Theme::from_json(json, false).unwrap();
        let rendered = renderer
            .render(&frame, Layout::Vertical, &theme, 2.0)
            .unwrap();
        rendered.material.unwrap()
    };
    let auto = THEME.replace(
        r#""kind": "glass""#,
        r#""kind": "glass", "appearance": "auto""#,
    );
    let followed = backing(&mut renderer, &auto);
    assert!(followed.follows_backdrop);
    assert_eq!(followed.locked_dark, None);

    let locked = auto.replace(
        r#""license": "MIT""#,
        r#""license": "MIT", "appearance": "light""#,
    );
    let pinned = backing(&mut renderer, &locked);
    assert!(!pinned.follows_backdrop);
    assert_eq!(pinned.locked_dark, Some(false));
}
