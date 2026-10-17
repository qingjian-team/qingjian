//! 截图模式：拷一份配置到输出目录（真配置不动），按脚本点几下，每步存一张 PNG。给没法亲手点的时候看效果用。

use std::path::Path;
use std::time::Instant;

use bamboo_slip::tiny_skia::IntRect;
use bamboo_slip::{App, Event, FontLibrary, Key, KeyCode, KeyPress, Modifiers, Runner, TextInput};

use crate::page_id::PageId;
use crate::settings::Settings;
use crate::support::Feature;

const SCALE: f32 = 2.0;

const SIZE: (u32, u32) = (1640, 1240);

pub fn run(config: Option<&Path>, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    let copy = directory.join("config.toml");
    match config.filter(|path| path.exists()) {
        Some(path) => {
            std::fs::copy(path, &copy)?;
        }
        None => {
            let _ = std::fs::remove_file(&copy);
        }
    }
    let settings = Settings::load(copy)?;
    let themes: Vec<String> = settings
        .themes
        .themes()
        .iter()
        .map(|theme| theme.id().to_owned())
        .collect();
    let mut runner = Runner::new(settings, FontLibrary::system("zh-CN")?);
    runner.stage().set_animations(false);
    runner.rebuild(SCALE, SIZE, Instant::now())?;

    let mut index = 0;
    let mut shot =
        |runner: &mut Runner<Settings>, caption: &str| -> Result<(), Box<dyn std::error::Error>> {
            let painted = runner.rebuild(SCALE, SIZE, Instant::now())?;
            let rect = IntRect::from_xywh(
                painted.content_x as i32,
                painted.content_y as i32,
                painted.content_width,
                painted.content_height,
            )
            .ok_or("截图区域无效")?;
            let pixmap = painted.pixmap.clone_rect(rect).ok_or("截图区域超出位图")?;
            index += 1;
            let path = directory.join(format!("{index:02}.png"));
            pixmap.save_png(&path)?;
            println!("{}：{caption}", path.display());
            Ok(())
        };

    for page in PageId::ALL {
        click(&mut runner, Key::ROOT.child("nav").child(page))?;
        shot(&mut runner, page.label())?;
        if page == PageId::About {
            click(&mut runner, Key::ROOT.child("sources"))?;
        }
        if matches!(page, PageId::About | PageId::Input) {
            let area = content(&runner);
            hover(&mut runner, area)?;
            runner.handle(&Event::Scroll {
                dx: 0.0,
                dy: 5000.0,
            });
            shot(&mut runner, &format!("{}：滚到底", page.label()))?;
        }
    }

    click(&mut runner, Key::ROOT.child("nav").child(PageId::Input))?;
    let area = content(&runner);
    hover(&mut runner, area)?;
    runner.handle(&Event::Scroll {
        dx: 0.0,
        dy: -5000.0,
    });
    click(&mut runner, Key::ROOT.child("scheme"))?;
    shot(&mut runner, "输入：点拼音下拉框")?;
    click(&mut runner, Key::ROOT.child("menu").child(1_usize))?;
    shot(&mut runner, "选小鹤双拼：多出「输入框显示按键」")?;

    click(&mut runner, Key::ROOT.child("nav").child(PageId::Phrases))?;
    click(&mut runner, Key::ROOT.child("add-phrase"))?;
    shot(&mut runner, "自定义短语：点「新增」弹出编辑卡片")?;
    click(&mut runner, Key::ROOT.child("phrase-save"))?;
    shot(&mut runner, "空着点保存：卡片里说原因")?;
    click(&mut runner, Key::ROOT.child("phrase-cancel"))?;

    click(&mut runner, Key::ROOT.child("nav").child(PageId::Advanced))?;
    click(&mut runner, Key::ROOT.child("clear-input-log"))?;
    shot(&mut runner, "高级：点一下清空，按钮变成确认")?;

    click(&mut runner, Key::ROOT.child("search"))?;
    runner.handle(&Event::Text(TextInput::Insert("模糊".to_owned())));
    shot(&mut runner, "搜索「模糊」：侧栏换成结果")?;
    runner.handle(&Event::Key(KeyPress::new(
        KeyCode::Enter,
        Modifiers::default(),
    )));
    shot(&mut runner, "回车：跳到第一条，那一行滚进视野并标出来")?;
    runner.handle(&Event::Key(KeyPress::new(
        KeyCode::Escape,
        Modifiers::default(),
    )));
    runner.handle(&Event::Text(TextInput::Insert("英语".to_owned())));
    shot(&mut runner, "搜索「英语」：控件上的字也搜得到")?;
    runner.handle(&Event::Key(KeyPress::new(
        KeyCode::Escape,
        Modifiers::default(),
    )));
    runner.rebuild(SCALE, SIZE, Instant::now())?;

    click(
        &mut runner,
        Key::ROOT.child("nav").child(PageId::Appearance),
    )?;
    if let Some(id) = themes.last() {
        click(&mut runner, Key::ROOT.child("themes").child(id))?;
    }
    click(&mut runner, Key::ROOT.child("appearance").child(2_usize))?;
    shot(&mut runner, "外观：最后一个主题、深色")?;
    runner.app_mut().set_system_dark(true);
    shot(&mut runner, "系统深色：设置窗口自己也换深色")?;
    click(&mut runner, Key::ROOT.child("nav").child(PageId::Language))?;
    shot(&mut runner, "中英文（深色）")?;
    runner.app_mut().set_system_dark(false);
    stats(&mut runner, &mut shot)?;
    Ok(())
}

/// 统计页两层，装上样例数据（真数据多半太少，看不出图表的样子）；各悬停一格、一根柱看浮出的说明。
fn stats(
    runner: &mut Runner<Settings>,
    shot: &mut impl FnMut(&mut Runner<Settings>, &str) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    click(runner, Key::ROOT.child("nav").child(PageId::Stats))?;
    let now = jiff::Zoned::now().datetime();
    let (input, cloud) = super::sample::stats(now.date(), now.hour() as u8);
    if let Some(stats) = runner.app_mut().stats.as_mut() {
        stats.input = input;
        stats.cloud = cloud;
    }
    runner.rebuild(SCALE, SIZE, Instant::now())?;
    let cell = |local: &str, column: usize, row: usize| {
        Key::ROOT.child((Key::ROOT.child(local), column, row))
    };
    let bar = |local: &str, hour: usize| Key::ROOT.child((Key::ROOT.child(local), hour));
    // 云服务那一栏只在支持云服务用量统计的平台上有（Linux 还没接），没有它就别去点
    let tabs = [
        ("输入", "input-days", "input-hours"),
        ("云服务", "cloud-days", "cloud-hours"),
    ];
    for (tab, days, hours) in tabs
        .into_iter()
        .filter(|(tab, ..)| *tab != "云服务" || Feature::ApiUsage.here())
    {
        runner.app_mut().stats_cloud = tab == "云服务";
        runner.rebuild(SCALE, SIZE, Instant::now())?;
        hover(runner, cell(days, 50, 2))?;
        runner.rebuild(SCALE, SIZE, Instant::now())?;
        shot(runner, &format!("统计·{tab}（样例数据）：悬停一格"))?;
        reveal(runner, bar(hours, 10))?;
        hover(runner, bar(hours, 10))?;
        runner.rebuild(SCALE, SIZE, Instant::now())?;
        shot(runner, &format!("统计·{tab}：过去 24 小时，悬停第 11 根"))?;
        let area = content(runner);
        hover(runner, area)?;
        runner.handle(&Event::Scroll {
            dx: 0.0,
            dy: 5000.0,
        });
        shot(runner, &format!("统计·{tab}：滚到底"))?;
        hover(runner, area)?;
        runner.handle(&Event::Scroll {
            dx: 0.0,
            dy: -5000.0,
        });
    }
    Ok(())
}

fn hover(runner: &mut Runner<Settings>, key: Key) -> Result<(), Box<dyn std::error::Error>> {
    let (x, y, width, height) = runner.interaction().rect(key).ok_or("找不到元素")?;
    runner.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    Ok(())
}

/// 当前页的滚动区。
fn content(runner: &Runner<Settings>) -> Key {
    Key::ROOT.child(("content", runner.app().page))
}

/// 元素在滚动区外时先滚到看得见（离视口边留 40 像素）；侧栏里的不用滚。
fn reveal(runner: &mut Runner<Settings>, key: Key) -> Result<(), Box<dyn std::error::Error>> {
    let content = content(runner);
    let Some((left, ..)) = runner.interaction().rect(content) else {
        return Ok(());
    };
    if runner
        .interaction()
        .rect(key)
        .is_some_and(|(x, ..)| x < left)
    {
        return Ok(());
    }
    let (_, top, _, height) = runner.interaction().rect(content).ok_or("找不到滚动区")?;
    let (_, y, _, h) = runner.interaction().rect(key).ok_or("找不到元素")?;
    let margin = 40.0;
    let dy = if y + h > top + height - margin {
        y + h - (top + height - margin)
    } else if y < top + margin {
        y - top - margin
    } else {
        return Ok(());
    };
    hover(runner, content)?;
    runner.handle(&Event::Scroll { dx: 0.0, dy });
    runner.rebuild(SCALE, SIZE, Instant::now())?;
    Ok(())
}

fn click(runner: &mut Runner<Settings>, key: Key) -> Result<(), Box<dyn std::error::Error>> {
    reveal(runner, key)?;
    hover(runner, key)?;
    runner.handle(&Event::PointerDown);
    runner.handle(&Event::PointerUp);
    runner.rebuild(SCALE, SIZE, Instant::now())?;
    Ok(())
}
