use taffy::{Dimension, FlexDirection, Size, Style};

use super::super::*;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Interaction, Ui};

#[derive(Debug, Clone, PartialEq)]
enum FuzzMsg {
    Toggle,
    Pick(usize),
    Change(f32),
}

/// 固定种子的线性同余随机数：同一份代码跑到哪次结果都一样，布局越界可复现。
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as u32 as f32) / (1u32 << 31) as f32
    }
}

#[test]
fn random_sizes_layout_without_panic_or_overflow() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let pool = ["一", "word", "长短不一的标签", "abc", "设置项"];

    for _ in 0..120 {
        let width = 160.0 + rng.next() * 280.0;
        let interaction = Interaction::<FuzzMsg>::new();
        let mut ui = Ui::new(&interaction);
        let word = pool[(rng.next() * (pool.len() - 1) as f32).round() as usize];
        let edit = TextEdit::new(word);
        let field = TextField::new(&edit)
            .stretch()
            .build(&mut ui, &theme, "field", |_| FuzzMsg::Toggle, None, None)
            .unwrap();
        let select = Select::new("选项")
            .options(&["一", "二"])
            .width(160.0)
            .build(&mut ui, &theme, "select", FuzzMsg::Toggle)
            .unwrap();
        let slider = Slider::new(rng.next(), 0.0, 1.0)
            .build(&mut ui, &theme, "slider", FuzzMsg::Change)
            .unwrap();
        let button = Button::new("按钮")
            .build(&mut ui, &theme, "button", FuzzMsg::Toggle)
            .unwrap();
        let switch = Switch::new(rng.next() > 0.5)
            .build(&mut ui, &theme, "switch", FuzzMsg::Toggle)
            .unwrap();
        let root = ui
            .scene()
            .node(
                Style {
                    size: Size {
                        width: Dimension::length(width),
                        height: Dimension::auto(),
                    },
                    flex_direction: FlexDirection::Column,
                    ..Style::default()
                },
                Visual::Group,
                &[field, select, slider, button, switch],
            )
            .unwrap();
        let (mut scene, _) = ui.finish();
        scene.layout(root, &mut text).unwrap();

        let located = scene.paint_order(root).unwrap();
        let (_, _, root_w, root_h) = scene.rect_in(root, root).unwrap();
        for place in located {
            let (x, y, w, h) = place.rect;
            assert!(x >= -0.5 && y >= -0.5, "负坐标 {:?}", place.rect);
            assert!(w >= -0.5 && h >= -0.5, "负尺寸 {:?}", place.rect);
            assert!(
                x + w <= root_w + 0.5,
                "横向越界 {:?}，根宽 {root_w}",
                place.rect
            );
            assert!(
                y + h <= root_h + 0.5,
                "纵向越界 {:?}，根高 {root_h}",
                place.rect
            );
        }
    }
}
