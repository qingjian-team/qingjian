//! 截图模式：不开窗口，按脚本发一串事件，每步存一张 PNG，最后拼一张总览（总览也用引擎画）。

use std::path::Path;
use std::sync::Arc;

use bamboo_slip::taffy::{Dimension, FlexDirection, FlexWrap, LengthPercentage, Rect, Size, Style};
use bamboo_slip::{
    BoxPaint, Color, Event, Fill, FontSpec, Key, KeyCode, KeyPress, Modifiers, Pixmap, Scene,
    Stage, TextStyle, Visual,
};

use std::time::Instant;

use bamboo_slip::{FontLibrary, Runner};

use crate::demo::Demo;

const SCALE: f32 = 2.0;

const SIZE: (u32, u32) = (1560, 1160);

/// 一步：说明，以及截图前要发的事件。
enum Step {
    Shot(&'static str),
    Move(Key),
    Down,
    Up,
    Press(KeyCode, bool),
    Scroll(f32),
}

pub fn run(directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    let mut runner = Runner::new(Demo::new(false), FontLibrary::system("zh-CN")?);
    // 截的是每步的终态
    runner.stage().set_animations(false);
    let nav = |index: usize| Key::ROOT.child("nav").child(index);
    let option = |choice: &str, index: usize| Key::ROOT.child(choice).child(index);
    let steps = [
        Step::Shot("默认"),
        Step::Move(Key::ROOT.child(("content", 1_usize))),
        Step::Scroll(120.0),
        Step::Shot("往下滚：标题滚出去，右边浮出滚动条"),
        Step::Scroll(5000.0),
        Step::Shot("滚到底：停在最后一组"),
        Step::Scroll(-5000.0),
        Step::Move(nav(4)),
        Step::Shot("悬停导航「模糊音」"),
        Step::Move(option("Layout", 1)),
        Step::Shot("悬停分段「横排」"),
        Step::Down,
        Step::Up,
        Step::Shot("点「横排」：选中块滑过去，展开多行可以勾了"),
        Step::Move(Key::ROOT.child("expand")),
        Step::Down,
        Step::Up,
        Step::Shot("勾上「展开成多行」"),
        Step::Move(Key::ROOT.child("Candidate").child("increase")),
        Step::Down,
        Step::Shot("按住候选字号的 +"),
        Step::Up,
        Step::Down,
        Step::Up,
        Step::Shot("点两下：19 pt"),
        Step::Press(KeyCode::Tab, false),
        Step::Shot("Tab：焦点环到译文字号"),
        Step::Press(KeyCode::ArrowUp, false),
        Step::Shot("↑：译文字号 14 pt"),
        Step::Move(Key::ROOT.child(("content", 1_usize))),
        Step::Scroll(5000.0),
        Step::Move(Key::ROOT.child("animations")),
        Step::Down,
        Step::Shot("按住开关：旋钮拉宽"),
        Step::Up,
        Step::Shot("松开：过渡动画关了"),
        Step::Move(option("Engine", 1)),
        Step::Down,
        Step::Up,
        Step::Shot("选「系统绘制」：字体与字号变灰"),
        Step::Scroll(-5000.0),
        Step::Move(option("Appearance", 2)),
        Step::Down,
        Step::Up,
        Step::Press(KeyCode::Tab, false),
        Step::Shot("外观选「深色」，再按 Tab"),
    ];

    let mut shots: Vec<(&str, Pixmap)> = Vec::new();
    // 先出一帧，交互层才有元素的位置
    runner.rebuild(SCALE, SIZE, Instant::now())?;
    for step in steps {
        let event = match step {
            Step::Shot(caption) => {
                let painted = runner.rebuild(SCALE, SIZE, Instant::now())?;
                let pixmap = painted
                    .pixmap
                    .clone_rect(
                        bamboo_slip::tiny_skia::IntRect::from_xywh(
                            painted.content_x as i32,
                            painted.content_y as i32,
                            painted.content_width,
                            painted.content_height,
                        )
                        .ok_or("截图区域无效")?,
                    )
                    .ok_or("截图区域超出位图")?;
                let path = directory.join(format!("{:02}.png", shots.len() + 1));
                pixmap.save_png(&path)?;
                println!("{}：{caption}", path.display());
                shots.push((caption, pixmap));
                continue;
            }
            Step::Move(key) => {
                let (x, y, width, height) = runner.interaction().rect(key).ok_or("找不到元素")?;
                Event::PointerMoved {
                    x: x + width / 2.0,
                    y: y + height / 2.0,
                }
            }
            Step::Scroll(dy) => Event::Scroll { dx: 0.0, dy },
            Step::Down => Event::PointerDown,
            Step::Up => Event::PointerUp,
            Step::Press(key, shift) => Event::Key(KeyPress::new(
                key,
                Modifiers {
                    shift,
                    ..Modifiers::default()
                },
            )),
        };
        if runner.handle(&event) {
            runner.rebuild(SCALE, SIZE, Instant::now())?;
        }
    }

    let sheet = sheet(runner.stage(), shots)?;
    let path = directory.join("sheet.png");
    sheet.save_png(&path)?;
    println!("{}：总览", path.display());
    Ok(())
}

/// 两列的总览：每格一张缩到一半的截图，上面写说明。
fn sheet(
    stage: &mut Stage,
    shots: Vec<(&str, Pixmap)>,
) -> Result<Pixmap, Box<dyn std::error::Error>> {
    let (cell_w, cell_h) = (SIZE.0 as f32 / 2.0, SIZE.1 as f32 / 2.0);
    let gap = 32.0;
    let mut scene = Scene::new();
    let caption = TextStyle::new(FontSpec::new(28.0, 40.0), 14.0, Color::gray(0, 220), 1.0);
    let mut cells = Vec::new();
    for (index, (text, pixmap)) in shots.into_iter().enumerate() {
        let label = scene.node(
            Style::default(),
            Visual::Text {
                text: format!("{}. {text}", index + 1),
                style: caption,
            },
            &[],
        )?;
        let image = scene.node(
            Style {
                size: Size {
                    width: Dimension::length(cell_w),
                    height: Dimension::length(cell_h),
                },
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: Some(Fill::Image {
                    pixmap: Arc::new(pixmap),
                    slice: None,
                    pixels_per_px: 2.0,
                }),
                border: Some((1.0, Color::gray(0, 40))),
                radius: 0.0,
            }),
            &[],
        )?;
        cells.push(scene.node(
            Style {
                flex_direction: FlexDirection::Column,
                gap: Size {
                    width: LengthPercentage::length(0.0),
                    height: LengthPercentage::length(8.0),
                },
                ..Style::default()
            },
            Visual::Group,
            &[label, image],
        )?);
    }
    let padding = LengthPercentage::length(gap);
    let root = scene.node(
        Style {
            size: Size {
                width: Dimension::length(cell_w * 2.0 + gap * 3.0),
                height: Dimension::auto(),
            },
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            gap: Size {
                width: LengthPercentage::length(gap),
                height: LengthPercentage::length(gap),
            },
            padding: Rect {
                left: padding,
                right: padding,
                top: padding,
                bottom: padding,
            },
            ..Style::default()
        },
        Visual::solid(Color::rgb(0xE4, 0xE4, 0xE8), 0.0),
        &cells,
    )?;
    let content = scene.layout(root, stage.text())?;
    Ok(stage.rasterize(&scene, root, content, 1.0)?.pixmap)
}
