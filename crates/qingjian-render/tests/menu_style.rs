//! 候选右键菜单按主题画：主题写了 `menu` 一节就用它的颜色与圆角，没写就用控件的浅色 / 深色。
//!
//! 内置主题里只有「墨岩」（`slate`）写了这一节——它是方角主题，菜单也跟着方。

use qingjian_render::{MenuStyle, Theme, ThemeLibrary};

fn library() -> ThemeLibrary {
    ThemeLibrary::load(None)
}

#[test]
fn a_theme_without_a_menu_section_uses_the_widget_defaults() {
    // 青简绿没写 `menu`：浅色 / 深色就是控件自己那套，圆角跟控件默认
    let theme = Theme::light();
    let style = theme.menu_style(false);
    assert_eq!(style, MenuStyle::system(false));
    assert_eq!(theme.menu_style(true), MenuStyle::system(true));
}

#[test]
fn slate_paints_the_menu_flat_like_the_rest_of_the_theme() {
    let dark = false;
    let theme = library().resolve("slate", dark);
    let style = theme.menu_style(dark);
    // 墨岩是方角主题：菜单卡片不能用控件默认的圆角
    assert_eq!(style.radius, Some(0.0), "墨岩的菜单要方角");

    // 字色取自墨岩自己的变量（#14232b，青蓝近黑），不是控件的浅色默认
    let system = MenuStyle::system(dark);
    assert_eq!(
        style.colors.text,
        qingjian_render::Color::rgba(0x14, 0x23, 0x2b, 0xff),
        "菜单字要跟着主题"
    );
    assert_ne!(style.colors.text, system.colors.text);
}

#[test]
fn a_user_themes_flat_menu_overrides_only_what_it_lists() {
    // 用户主题只写圆角：颜色退回控件默认
    let theme = Theme::from_json(
        r#"{
            "schema": 1,
            "meta": { "id": "flat", "name": "方" },
            "menu": { "radius": 0 }
        }"#,
        false,
    )
    .expect("主题能读");
    let style = theme.menu_style(false);
    assert_eq!(style.radius, Some(0.0));
    assert_eq!(
        style.colors,
        MenuStyle::system(false).colors,
        "没写的颜色退回默认"
    );
}
