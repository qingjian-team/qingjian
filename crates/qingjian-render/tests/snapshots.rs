//! 快照测试：样例帧（`scenes/`）按浅 / 深色画成位图，与仓库里的基准 PNG 逐像素比，重构渲染器时防回退。
//!
//! 基准按平台放在 `tests/snapshots/<os>/`，旁边的 `fingerprint.txt` 记下出图时的字体环境（locale、字体文件与大小）。
//! 没有系统字体（CI 容器）、这个平台没有基准、字体环境与基准不一致（别的系统版本）时跳过，不算失败。
//!
//! 更新基准：`QINGJIAN_UPDATE_SNAPSHOTS=1 cargo test -p qingjian-render --test snapshots`，变了的 PNG 人工看过再提交。
//! 失败时实际图、基准图与差异图（不同的像素标红）写到 `target/tmp/render-snapshots/`。

mod scenes;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use qingjian_render::{
    FontLibrary, Frame, Layout, Mode, Pixmap, Rendered, Renderer, TextSizes, Theme,
};

const UPDATE_ENV: &str = "QINGJIAN_UPDATE_SNAPSHOTS";

const LOCALE: &str = "zh-CN";

/// 一张快照：名字、位图、写进 `metrics.txt` 的一行（位图尺寸、内容区、状态条格边界）。
struct Shot {
    name: String,
    pixmap: Pixmap,
    metrics: String,
}

#[test]
fn renderer_matches_snapshots() {
    let Ok(library) = FontLibrary::system(LOCALE) else {
        eprintln!("没有系统字体，跳过快照测试");
        return;
    };
    let fingerprint = fingerprint(&library);
    let dir = snapshot_dir();
    let update = std::env::var_os(UPDATE_ENV).is_some();
    if !update {
        match std::fs::read_to_string(dir.join("fingerprint.txt")) {
            Ok(saved) if saved == fingerprint => {}
            Ok(_) => {
                eprintln!(
                    "字体环境与基准不一致，跳过快照测试；确认渲染没变后用 {UPDATE_ENV}=1 重新出基准"
                );
                return;
            }
            Err(_) => {
                eprintln!("{} 没有基准，跳过快照测试", dir.display());
                return;
            }
        }
    }

    let shots = render_all(Renderer::new(library));
    let metrics: String = shots.iter().map(|shot| shot.metrics.clone()).collect();

    if update {
        std::fs::create_dir_all(&dir).unwrap();
        for shot in &shots {
            shot.pixmap
                .save_png(dir.join(format!("{}.png", shot.name)))
                .unwrap();
        }
        std::fs::write(dir.join("metrics.txt"), &metrics).unwrap();
        std::fs::write(dir.join("fingerprint.txt"), &fingerprint).unwrap();
        eprintln!("已更新 {} 张快照：{}", shots.len(), dir.display());
        return;
    }

    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("render-snapshots");
    let _ = std::fs::remove_dir_all(&out);
    let mut failures = Vec::new();
    for shot in &shots {
        if let Some(reason) = compare(shot, &dir, &out) {
            failures.push(format!("{}：{reason}", shot.name));
        }
    }
    let saved_metrics = std::fs::read_to_string(dir.join("metrics.txt")).unwrap_or_default();
    if saved_metrics != metrics {
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("metrics.txt"), &metrics).unwrap();
        failures.push("metrics.txt 与基准不同（尺寸 / 内容区 / 格边界）".to_owned());
    }
    assert!(
        failures.is_empty(),
        "{} 处与基准不同，实际图与差异图在 {}：\n{}",
        failures.len(),
        out.display(),
        failures.join("\n")
    );
}

/// 全部样例：候选窗浅 / 深色 × 各帧（2 倍），其余内置主题各几张，1 倍屏一张，状态条浅 / 深色。
fn render_all(mut renderer: Renderer) -> Vec<Shot> {
    let mut shots = Vec::new();
    for (theme_name, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
        for (scene, frame, layout) in scenes::candidate_scenes() {
            let rendered = still(&mut renderer, &frame, layout, &theme, 2.0);
            shots.push(shot(format!("{scene}-{theme_name}"), rendered, None));
        }
        let status = renderer
            .render_status(&scenes::status_cells(), &Mode::default(), &theme, 2.0)
            .unwrap();
        shots.push(shot(
            format!("status-{theme_name}"),
            status.rendered,
            Some(&status.cell_edges),
        ));
    }
    // 其余内置主题：竖排与横排云端各一张，看高亮换色
    for id in ["system-blue", "wechat"] {
        for (theme_name, dark) in [("light", false), ("dark", true)] {
            let theme = Theme::builtin(id, dark).unwrap();
            for (scene, frame, layout) in [
                ("nihao-vertical", scenes::nihao(), Layout::Vertical),
                ("cloud-horizontal", scenes::cloud(), Layout::Horizontal),
            ] {
                let rendered = still(&mut renderer, &frame, layout, &theme, 2.0);
                shots.push(shot(format!("{id}-{scene}-{theme_name}"), rendered, None));
            }
        }
    }
    // 能力展示主题（tests/themes/showcase）：九宫格图片、渐变、边框、半透明装饰
    let showcase = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/themes/showcase");
    for (theme_name, dark) in [("light", false), ("dark", true)] {
        let theme = Theme::from_dir(&showcase, dark).unwrap();
        for (scene, frame, layout) in [
            ("nihao-vertical", scenes::nihao(), Layout::Vertical),
            ("cloud-horizontal", scenes::cloud(), Layout::Horizontal),
        ] {
            let rendered = still(&mut renderer, &frame, layout, &theme, 2.0);
            shots.push(shot(
                format!("showcase-{scene}-{theme_name}"),
                rendered,
                None,
            ));
        }
    }
    shots.push(transition_mid(&mut renderer));
    shots.push(loop_frame(&mut renderer, &showcase));
    let rendered = still(
        &mut renderer,
        &scenes::nihao(),
        Layout::Vertical,
        &Theme::light(),
        1.0,
    );
    shots.push(shot("nihao-vertical-light-1x".to_owned(), rendered, None));
    // 设置里放大字号：行高跟着撑开，候选与译文不叠在一起
    let large = Theme::light().with_text_sizes(TextSizes::new(24.0, 16.0));
    for (scene, frame, layout) in [
        ("nihao-vertical", scenes::nihao(), Layout::Vertical),
        ("cloud-horizontal", scenes::cloud(), Layout::Horizontal),
    ] {
        let rendered = still(&mut renderer, &frame, layout, &large, 2.0);
        shots.push(shot(format!("large-text-{scene}-light"), rendered, None));
    }
    // 候选与译文按不同比例改字号：一行里各格按基线对齐
    for (name, sizes) in [
        ("large-candidate", TextSizes::new(32.0, 0.0)),
        ("large-annotation", TextSizes::new(0.0, 22.0)),
    ] {
        let theme = Theme::light().with_text_sizes(sizes);
        let rendered = still(
            &mut renderer,
            &scenes::nihao(),
            Layout::Vertical,
            &theme,
            2.0,
        );
        shots.push(shot(format!("{name}-nihao-vertical-light"), rendered, None));
    }
    shots
}

/// 静止的一帧：先忘掉上一张，主题里的过渡不和上一张样例配对。
fn still(
    renderer: &mut Renderer,
    frame: &Frame,
    layout: Layout,
    theme: &Theme,
    scale: f32,
) -> Rendered {
    renderer.forget();
    renderer.render(frame, layout, theme, scale).unwrap()
}

/// 过渡（青简绿的高亮条）：高亮从第 1 项移到第 4 项，取 60 ms 处的一帧当快照；播完的最后一帧要与直接画第 4 项高亮逐像素一致。
fn transition_mid(renderer: &mut Renderer) -> Shot {
    let theme = Theme::light();
    let from = scenes::nihao();
    let mut to = scenes::nihao();
    to.highlighted = Some(3);
    let start = Instant::now();
    // 设置里关了动画：换行直接画终点，不要后续帧
    let off = theme.with_animations(false);
    renderer.forget();
    renderer
        .render_at(&from, Layout::Vertical, &off, 2.0, start)
        .unwrap();
    let jumped = renderer
        .render_at(&to, Layout::Vertical, &off, 2.0, start)
        .unwrap();
    assert!(jumped.next_frame.is_none(), "关了动画不该过渡");
    renderer.forget();
    renderer
        .render_at(&from, Layout::Vertical, &theme, 2.0, start)
        .unwrap();
    let first = renderer
        .render_at(&to, Layout::Vertical, &theme, 2.0, start)
        .unwrap();
    assert!(first.next_frame.is_some(), "高亮换行应当开始过渡");
    let mid = renderer
        .tick_at(start + Duration::from_millis(60))
        .unwrap()
        .expect("过渡中途应有新一帧");
    let end = renderer
        .tick_at(start + Duration::from_millis(200))
        .unwrap()
        .expect("播完的那一帧");
    assert!(end.next_frame.is_none(), "播完不再要帧");
    assert!(
        renderer
            .tick_at(start + Duration::from_millis(300))
            .unwrap()
            .is_none()
    );
    let still = still(renderer, &to, Layout::Vertical, &theme, 2.0);
    assert!(end.pixmap == still.pixmap, "过渡的最后一帧与直接画的不一致");
    shot("transition-mid".to_owned(), mid, None)
}

/// 循环动画（展示主题的两片花瓣）：取 600 ms 处的一帧当快照；减少动态效果时停在开头、不再要帧。
fn loop_frame(renderer: &mut Renderer, showcase: &Path) -> Shot {
    let theme = Theme::from_dir(showcase, false).unwrap();
    let frame = scenes::nihao();
    let start = Instant::now();
    renderer.forget();
    let first = renderer
        .render_at(&frame, Layout::Vertical, &theme, 2.0, start)
        .unwrap();
    assert!(first.next_frame.is_some(), "有循环动画应当接着要帧");
    let later = renderer
        .tick_at(start + Duration::from_millis(600))
        .unwrap()
        .expect("循环动画的一帧");
    assert!(later.next_frame.is_some(), "循环动画一直播");
    // 循环帧走局部重画（各段图层叠放），与同一时刻整张重画只能差在半透明叠加的取整上（实测最多 3 / 255）
    let full = renderer
        .render_at(
            &frame,
            Layout::Vertical,
            &theme,
            2.0,
            start + Duration::from_millis(600),
        )
        .unwrap();
    let worst = max_channel_difference(&later.pixmap, &full.pixmap);
    assert!(worst <= 3, "局部重画与整张重画差太多：{worst}");
    assert_eq!(
        (later.pixmap.width(), later.pixmap.height()),
        (first.pixmap.width(), first.pixmap.height()),
        "动画中位图不变大小"
    );
    renderer.set_reduce_motion(true);
    renderer.forget();
    let calm = renderer
        .render(&frame, Layout::Vertical, &theme, 2.0)
        .unwrap();
    renderer.set_reduce_motion(false);
    assert!(calm.next_frame.is_none(), "减少动态效果时不播循环动画");
    assert!(calm.pixmap == first.pixmap, "减少动态效果时停在第一帧");
    shot("showcase-loop".to_owned(), later, None)
}

fn shot(name: String, rendered: Rendered, cell_edges: Option<&[f32]>) -> Shot {
    let mut metrics = format!(
        "{name}  bitmap {}x{}  content {},{} {}x{}",
        rendered.pixmap.width(),
        rendered.pixmap.height(),
        rendered.content_x,
        rendered.content_y,
        rendered.content_width,
        rendered.content_height,
    );
    if let Some(edges) = cell_edges {
        let _ = write!(metrics, "  edges {edges:?}");
    }
    metrics.push('\n');
    Shot {
        name,
        pixmap: rendered.pixmap,
        metrics,
    }
}

/// 与基准比；不同就把实际图、基准图、差异图写到 `out`，返回原因。
fn compare(shot: &Shot, dir: &Path, out: &Path) -> Option<String> {
    let file = format!("{}.png", shot.name);
    let Ok(expected) = Pixmap::load_png(dir.join(&file)) else {
        return Some("没有基准图".to_owned());
    };
    // 存 PNG 要去预乘，半透明像素会有取整；实际图也过一遍 PNG 再比，两边取整一致
    let actual = Pixmap::decode_png(&shot.pixmap.encode_png().unwrap()).unwrap();
    let reason = if (actual.width(), actual.height()) != (expected.width(), expected.height()) {
        format!(
            "尺寸 {}x{}，基准 {}x{}",
            actual.width(),
            actual.height(),
            expected.width(),
            expected.height()
        )
    } else {
        let differing = actual
            .pixels()
            .iter()
            .zip(expected.pixels())
            .filter(|(a, b)| a != b)
            .count();
        if differing == 0 {
            return None;
        }
        format!("{differing} 个像素不同")
    };
    std::fs::create_dir_all(out).unwrap();
    let _ = actual.save_png(out.join(&file));
    let _ = expected.save_png(out.join(format!("{}.expected.png", shot.name)));
    if let Some(diff) = diff_image(&actual, &expected) {
        let _ = diff.save_png(out.join(format!("{}.diff.png", shot.name)));
    }
    Some(reason)
}

/// 两张同大位图逐像素各通道差的最大值。
fn max_channel_difference(a: &Pixmap, b: &Pixmap) -> u8 {
    assert_eq!((a.width(), a.height()), (b.width(), b.height()));
    a.data()
        .iter()
        .zip(b.data())
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0)
}

/// 差异图：不同的像素标红，其余画成实际图的淡灰剪影；尺寸不同时不出。
fn diff_image(actual: &Pixmap, expected: &Pixmap) -> Option<Pixmap> {
    if (actual.width(), actual.height()) != (expected.width(), expected.height()) {
        return None;
    }
    let mut diff = Pixmap::new(actual.width(), actual.height())?;
    let red = tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 0, 255)?;
    for ((out, a), b) in diff
        .pixels_mut()
        .iter_mut()
        .zip(actual.pixels())
        .zip(expected.pixels())
    {
        *out = if a == b {
            let alpha = a.alpha() / 4;
            tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, alpha)?
        } else {
            red
        };
    }
    Some(diff)
}

/// 字体环境：locale 与加载到的字体文件（路径 + 大小）。系统升级换了字体文件，大小一般会变。
fn fingerprint(library: &FontLibrary) -> String {
    let mut text = format!("locale {LOCALE}\n");
    for path in library.font_files() {
        let size = std::fs::metadata(&path).map_or(0, |meta| meta.len());
        let _ = writeln!(text, "{size} {}", path.display());
    }
    text
}

fn snapshot_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(std::env::consts::OS)
}
