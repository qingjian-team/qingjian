//! 方角主题（墨岩）要把圆角一路传到每个显示面：窗口、符号/翻译面板、状态条、菜单。

use qingjian_render::Theme;

/// 墨岩：所有显示面的圆角都是 0，一条都不许漏。
#[test]
fn slate_is_square_everywhere() {
    let theme = Theme::builtin("slate", true).expect("内置主题里有墨岩");
    assert_eq!(theme.window_radius(), 0.0, "候选窗口根节点");
    let corners = theme.panel_corners();
    assert_eq!(corners.radius, 0.0, "符号 / 翻译 / 搜索 / 表情面板");
    assert_eq!(corners.cell_radius, 0.0, "面板里的格子");
}

/// 圆角主题不受影响：缺省的面板仍然带圆角。
#[test]
fn rounded_themes_keep_their_corners() {
    let theme = Theme::builtin("qingjian", true).expect("内置主题里有青简");
    assert!(theme.window_radius() > 0.0, "候选窗口");
    assert!(theme.panel_corners().radius > 0.0, "面板");
}
