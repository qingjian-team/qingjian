use taffy::{FlexDirection, Style};

use super::super::*;
use super::Message;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::ui::{Interaction, Ui};

/// 上面空出 `above` 像素，下面一个开关、一个勾选框；交给舞台画，返回还有没有过渡在播。
fn shifted(stage: &mut crate::Stage, above: f32, on: bool, now: std::time::Instant) -> bool {
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut interaction = Interaction::<Message>::new();
    let mut ui = Ui::new(&interaction);
    let spacer = ui
        .scene()
        .node(
            Style {
                size: taffy::Size {
                    width: taffy::Dimension::length(10.0),
                    height: taffy::Dimension::length(above),
                },
                ..Style::default()
            },
            Visual::Group,
            &[],
        )
        .unwrap();
    let switch = Switch::new(on)
        .build(&mut ui, &theme, "switch", Message::Toggle)
        .unwrap();
    let checkbox = Checkbox::new(on, "勾")
        .build(&mut ui, &theme, "check", Message::Toggle)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Column,
                ..Style::default()
            },
            Visual::Group,
            &[spacer, switch, checkbox],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    let size = scene.layout(root, stage.text()).unwrap();
    interaction.attach(&scene, root, elements).unwrap();
    stage
        .present(scene, root, size, 0, 1.0, now)
        .unwrap()
        .next_frame
        .is_some()
}

#[test]
fn controls_pushed_by_layout_jump_while_their_own_parts_still_slide() {
    let mut stage = crate::Stage::new(FontLibrary::system("en").unwrap());
    let start = std::time::Instant::now();
    assert!(!shifted(&mut stage, 0.0, false, start));
    let later = start + std::time::Duration::from_secs(1);
    assert!(
        !shifted(&mut stage, 40.0, false, later),
        "整体被挤下去不该有过渡"
    );
    assert!(shifted(&mut stage, 40.0, true, later), "拨开关时圆钮要滑");
}
