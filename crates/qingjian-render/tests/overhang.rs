//! 看得见的投影伸出量：壳避让投影按它算，比位图留的边（模糊半径的理论范围）小，又不为零。

mod scenes;

use qingjian_render::{FontLibrary, Layout, Renderer, Theme};

#[test]
fn visible_overhang_is_the_shadow_you_can_see_not_the_whole_margin() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let theme = Theme::from_json(
        r#"{ "extends": "qingjian", "schema": 1, "meta": { "id": "t", "name": "t" } }"#,
        false,
    )
    .unwrap();
    let mut renderer = Renderer::new(library);
    for layout in [Layout::Vertical, Layout::Horizontal] {
        let rendered = renderer
            .render(&scenes::nihao(), layout, &theme, 2.0)
            .unwrap();
        let margin_above = rendered.content_y;
        let margin_below = rendered.pixmap.height() - rendered.content_y - rendered.content_height;
        let (above, below) = rendered.visible_overhang();
        eprintln!(
            "{layout:?}: 留边 上 {margin_above} 下 {margin_below}，看得见 上 {above} 下 {below}"
        );
        assert!(
            below > 0 && below < margin_below,
            "下沿投影看得见，但不到留的边"
        );
        assert!(above < margin_above, "投影往下偏，上沿看得见的更少");
        assert!(below > above);
    }
}
