//! 离线预览：`cargo run --release -p qingjian-render --example preview -- --out target/render-preview`
//! 把样例帧按浅 / 深色、竖 / 横排画成 PNG，与各平台原生候选窗截图并排比；`--measure` 只量几段文字的宽度与原生对数；
//! 末尾列出验收行每个字形落到了哪家字体。不是日常工具，改渲染器时拿来核对。
//! `--theme <主题目录>` 画用户主题；主题样式写的系统字族在 `--font-dir`（可写多个）里按字族名找文件，代替壳的系统字体登记。

use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::Parser;
use qingjian_render::{FontLibrary, Mode, Renderer, Theme};

#[path = "../tests/scenes/mod.rs"]
mod scenes;

#[derive(Parser)]
struct Args {
    /// PNG 输出目录。
    #[arg(long, default_value = "target/render-preview")]
    out: PathBuf,

    /// 点 → 像素倍数（Retina 为 2）。
    #[arg(long, default_value_t = 2.0)]
    scale: f32,

    /// 中日字形回退用的 locale。
    #[arg(long, default_value = "zh-CN")]
    locale: String,

    /// 只量几段文字的宽度（点），不出图；与 AppKit 的 NSAttributedString.size() 对数。
    #[arg(long)]
    measure: bool,

    /// 画这个主题目录（含 theme.json），不写画缺省内置主题。
    #[arg(long)]
    theme: Option<PathBuf>,

    /// 主题写的系统字族到这些目录里找字体文件。
    #[arg(long = "font-dir", default_values = default_font_dirs())]
    font_dirs: Vec<PathBuf>,
}

/// 各平台放系统字体的目录。
fn default_font_dirs() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec![
            "/System/Library/Fonts",
            "/System/Library/Fonts/Supplemental",
            "/Library/Fonts",
        ]
    } else if cfg!(target_os = "windows") {
        vec![r"C:\Windows\Fonts"]
    } else {
        vec!["/usr/share/fonts"]
    }
}

/// 在字体目录里按字族名找文件（只看名字表，不加载）。
fn family_files(dirs: &[PathBuf], family: &str) -> Vec<PathBuf> {
    let mut db = cosmic_text::fontdb::Database::new();
    for dir in dirs {
        db.load_fonts_dir(dir);
    }
    let mut files = Vec::new();
    for face in db.faces() {
        let cosmic_text::fontdb::Source::File(path) = &face.source else {
            continue;
        };
        if face
            .families
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case(family))
            && !files.contains(path)
        {
            files.push(path.clone());
        }
    }
    files
}

fn load_theme(dir: &Path, dark: bool) -> Result<Theme, Box<dyn std::error::Error>> {
    Ok(Theme::from_dir(dir, dark)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "qingjian_render=debug".into()),
        )
        .init();
    let args = Args::parse();
    std::fs::create_dir_all(&args.out)?;

    let started = Instant::now();
    let library = FontLibrary::system(&args.locale)?;
    println!(
        "字体库：{:?}，界面字体 {}",
        started.elapsed(),
        library.ui_family()
    );
    println!("已加载字族：{}", library.families().join(" / "));
    let mut renderer = Renderer::new(library);
    if args.measure {
        for text in [
            "int. hello · int. hi",
            "hello",
            "ni'hao",
            "1/6",
            "你好",
            "phr. you change",
            "int. ",
            "·",
            " · ",
            "hi",
            "你好像",
            "開発する",
        ] {
            let widths: Vec<String> = [11.0, 12.0, 16.0]
                .into_iter()
                .map(|size| format!("{size}pt={:.2}", renderer.measure_points(text, size)))
                .collect();
            println!("{text:<24} {}", widths.join("  "));
        }
        return Ok(());
    }
    let themes = match &args.theme {
        Some(dir) => {
            let themes = [
                ("light", load_theme(dir, false)?),
                ("dark", load_theme(dir, true)?),
            ];
            renderer.load_theme_fonts(&themes[0].1, |family| family_files(&args.font_dirs, family));
            themes
        }
        None => [("light", Theme::light()), ("dark", Theme::dark())],
    };
    let samples = scenes::candidate_scenes();
    for (theme_name, theme) in &themes {
        for (scene, frame, layout) in &samples {
            let started = Instant::now();
            // 每张样例独立：不和上一张配对播过渡
            renderer.forget();
            let rendered = renderer.render(frame, *layout, theme, args.scale)?;
            let elapsed = started.elapsed();
            let path = args.out.join(format!("{scene}-{theme_name}.png"));
            rendered.pixmap.save_png(&path)?;
            let (w, h) = rendered.content_size_points();
            println!(
                "{:<28} {:>4.0}×{:<4.0}pt  {:>8.2?}  {}",
                format!("{scene}-{theme_name}"),
                w,
                h,
                elapsed,
                path.display()
            );
        }
    }

    // Windows 的悬浮状态条：三格
    let cells = scenes::status_cells();
    for (theme_name, theme) in &themes {
        let status = renderer.render_status(&cells, &Mode::default(), theme, args.scale)?;
        let path = args.out.join(format!("status-{theme_name}.png"));
        status.rendered.pixmap.save_png(&path)?;
        let (w, h) = status.rendered.content_size_points();
        println!(
            "{:<28} {:>4.0}×{:<4.0}pt  格边界 {:?}  {}",
            format!("status-{theme_name}"),
            w,
            h,
            status.cell_edges,
            path.display()
        );
    }

    for probe in [
        "青简 hello 🙂 日本語 骨直曜",
        "開発(かいはつ)する",
        "int. hello · int. hi",
    ] {
        println!(
            "「{probe}」各字形字体：{}",
            renderer.trace_families(probe, &themes[0].1).join(" → ")
        );
    }
    Ok(())
}
