//! 本机排版验收：复用真实候选窗和设置页，输出 PNG；不初始化输入引擎、不注册输入法、不读写用户配置。
//! `cargo run -p qingjian-macos --example typography_preview -- target/typography-preview`

#![allow(dead_code, unused_imports)]

#[path = "../src/app/mod.rs"]
mod app;
#[path = "../src/candidates/mod.rs"]
mod candidates;
#[path = "../src/error.rs"]
mod error;
#[path = "../src/host/mod.rs"]
mod host;
#[path = "../src/imk/mod.rs"]
mod imk;
#[path = "../src/menubar/mod.rs"]
mod menubar;
#[path = "../src/preferences/mod.rs"]
mod preferences;

use std::path::Path;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, Message};
use objc2_app_kit::{NSApplication, NSBitmapImageFileType, NSColorWell, NSTabView, NSView};
use objc2_foundation::{NSDate, NSDictionary, NSPoint, NSRect, NSRunLoop, NSSize};
use qingjian_core::{Candidate, CandidateKind, Language, PartOfSpeech, Sense, Translation};
use qingjian_platform::{CandidateRenderer, Config, GeneralConfig, LayoutMode, ThemeMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/typography-preview".into());
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.finishLaunching();
    let mut rows: Vec<_> = [
        ("中文候选词", "candidate"),
        ("学习", "language"),
        ("青简输入法", "Qingjian"),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (text, translation))| {
        candidates::Row::from_candidate(
            i,
            &Candidate {
                text: text.into(),
                kind: CandidateKind::Chinese,
                syllables: Vec::new(),
                reading: None,
                aux_code: None,
                translation: Some(Translation::new(
                    Language::English,
                    vec![Sense {
                        text: translation.into(),
                        part_of_speech: Some(PartOfSpeech::Verb),
                        reading: None,
                        fresh: i == 1,
                    }],
                )),
            },
        )
    })
    .collect();
    // 同一页混入英文候选与中文释义，字体必须按每一行的语言切换。
    let english = candidates::Row::from_candidate(
        3,
        &Candidate {
            text: "learning".into(),
            kind: CandidateKind::English,
            syllables: vec![],
            reading: None,
            aux_code: None,
            translation: Some(Translation::new(
                Language::Chinese,
                vec![Sense {
                    text: "学习".into(),
                    part_of_speech: Some(PartOfSpeech::Verb),
                    reading: None,
                    fresh: false,
                }],
            )),
        },
    );
    rows.push(english);
    // Core 的 emoji 标签写在 reading 中；中文提示和英文提示都要保留正确的字体。
    for (emoji, label) in [("🔶", "大"), ("😀", "smile")] {
        rows.push(candidates::Row::from_candidate(
            rows.len(),
            &Candidate {
                text: emoji.into(),
                kind: CandidateKind::Emoji,
                syllables: vec![],
                reading: Some(label.into()),
                translation: None,
                aux_code: None,
            },
        ));
    }
    let mut window = candidates::CandidateWindow::new(mtm);
    for (scenario, cn_size, en_size, pos_size, cn_bold, en_bold) in [
        ("split", 24, 18, 12, true, false),
        ("regular", 24, 18, 12, false, false),
        ("large-translation", 8, 48, 8, false, true),
        ("large-candidate", 48, 8, 8, true, false),
        ("large-pos", 18, 20, 48, true, false),
    ] {
        let general = GeneralConfig {
            candidate_font: "Songti SC".into(),
            font: "BM Dohyeon".into(),
            candidate_font_size: cn_size,
            annotation_font_size: en_size,
            pos_font_size: pos_size,
            candidate_bold: cn_bold,
            annotation_bold: en_bold,
            ..GeneralConfig::default()
        };
        window.set_typography(&candidates::typography::Typography::from(&general));
        for (backend, renderer) in [
            ("qingjian", CandidateRenderer::Qingjian),
            ("system", CandidateRenderer::System),
        ] {
            window.set_renderer(renderer);
            for (layout_name, layout, columns) in [
                ("vertical", LayoutMode::Vertical, 0),
                ("horizontal", LayoutMode::Horizontal, 0),
                ("matrix", LayoutMode::Horizontal, 2),
            ] {
                window.set_layout(layout);
                window.set_theme(ThemeMode::Light);
                let frame = candidates::Frame {
                    rows: rows.clone(),
                    columns,
                    highlighted: 4,
                    footer: Some("1/3".into()),
                    ..candidates::Frame::default()
                };
                window.show(
                    frame.clone(),
                    NSRect::new(NSPoint::new(200.0, 650.0), NSSize::new(1.0, 16.0)),
                );
                let panel = app
                    .windows()
                    .iter()
                    .find(|window| window.isVisible())
                    .unwrap();
                save(
                    &panel.contentView().unwrap(),
                    &out.join(format!("{scenario}-{backend}-{layout_name}.png")),
                )?;
                if scenario == "split" && columns == 0 && layout == LayoutMode::Vertical {
                    window.set_theme(ThemeMode::Dark);
                    window.show(
                        frame,
                        NSRect::new(NSPoint::new(200.0, 650.0), NSSize::new(1.0, 16.0)),
                    );
                    save(
                        &panel.contentView().unwrap(),
                        &out.join(format!("split-{backend}-dark.png")),
                    )?;
                }
                window.hide();
            }
        }
    }
    // 两种引擎、三种排布、两种外观都使用同一套自定义配色；第二行是生词。
    let colors = sample_colors();
    window.set_colors(&candidates::colors::CandidateColors::from(&colors));
    window.set_typography(&candidates::typography::Typography::default());
    for (backend, renderer) in [
        ("qingjian", CandidateRenderer::Qingjian),
        ("system", CandidateRenderer::System),
    ] {
        window.set_renderer(renderer);
        for (layout_name, layout, columns) in [
            ("vertical", LayoutMode::Vertical, 0),
            ("horizontal", LayoutMode::Horizontal, 0),
            ("matrix", LayoutMode::Horizontal, 2),
        ] {
            window.set_layout(layout);
            for (appearance, theme) in [("light", ThemeMode::Light), ("dark", ThemeMode::Dark)] {
                window.set_theme(theme);
                window.show(
                    candidates::Frame {
                        rows: rows.clone(),
                        columns,
                        highlighted: 1,
                        ..candidates::Frame::default()
                    },
                    NSRect::new(NSPoint::new(200.0, 650.0), NSSize::new(1.0, 16.0)),
                );
                let panel = app
                    .windows()
                    .iter()
                    .find(|window| window.isVisible())
                    .unwrap();
                save(
                    &panel.contentView().unwrap(),
                    &out.join(format!("colors-{backend}-{layout_name}-{appearance}.png")),
                )?;
                window.hide();
            }
        }
    }
    let settings = preferences::PreferencesWindow::new(
        mtm,
        &[Language::English],
        "0.1.4-local-fonts",
        "排版预览",
    );
    let mut config = Config::default();
    config.general.candidate_font = "Songti SC".into();
    config.general.font = "BM Dohyeon".into();
    config.general.candidate_font_size = 24;
    config.general.annotation_font_size = 18;
    config.general.candidate_bold = true;
    settings.sync(
        &config,
        false,
        None,
        &[],
        &preferences::UpdateStatus::default(),
    );
    settings.show();
    let panel = app
        .windows()
        .iter()
        .find(|window| window.isVisible())
        .unwrap();
    let content = panel.contentView().unwrap();
    select_candidates(&content);
    NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.3));
    check_settings_preview(&settings, &content, out)?;
    check_color_controls(&settings, &content, out)?;
    settings.sync(
        &config,
        false,
        None,
        &[],
        &preferences::UpdateStatus::default(),
    );
    save(&content, &out.join("settings.png"))?;
    if std::env::args().any(|arg| arg == "--show-settings") {
        app.run();
    }
    panel.orderOut(None);
    println!("候选窗口与设置页预览验收通过：{}", out.display());
    Ok(())
}

fn select_candidates(view: &NSView) {
    if let Some(tabs) = view.downcast_ref::<NSTabView>() {
        tabs.selectTabViewItemAtIndex(1);
    }
    for child in view.subviews() {
        select_candidates(&child);
    }
}

/// 经过实际设置页的 sync 通路，检查两套引擎下每一组设置都能刷新预览。
fn check_settings_preview(
    settings: &preferences::PreferencesWindow,
    content: &NSView,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let preview = find_candidate_view(content).ok_or("设置页缺少候选字体预览")?;
    for (backend, renderer) in [
        ("qingjian", CandidateRenderer::Qingjian),
        ("system", CandidateRenderer::System),
    ] {
        let mut baseline = Config::default();
        baseline.general.candidate_font = "PingFang SC".into();
        baseline.general.font = "Helvetica".into();
        baseline.general.candidate_font_size = 18;
        baseline.general.annotation_font_size = 15;
        baseline.general.renderer = renderer;
        baseline.general.theme = ThemeMode::Light;
        settings.sync(
            &baseline,
            false,
            None,
            &[],
            &preferences::UpdateStatus::default(),
        );
        let original = png(&preview)?;
        save(
            &preview,
            &out.join(format!("settings-preview-{backend}-baseline.png")),
        )?;
        for scenario in [
            "candidate-font",
            "annotation-font",
            "candidate-size",
            "annotation-size",
            "pos-size",
            "candidate-bold",
            "annotation-bold",
            "maximum",
            "minimum",
            "dark",
            "background-color",
            "candidate-color",
            "pos-color",
            "word-color",
            "fresh-word-color",
            "highlight-color",
        ] {
            let mut changed = baseline.clone();
            match scenario {
                "candidate-font" => changed.general.candidate_font = "Songti SC".into(),
                "annotation-font" => changed.general.font = "Times New Roman".into(),
                "candidate-size" => changed.general.candidate_font_size = 28,
                "annotation-size" => changed.general.annotation_font_size = 28,
                "pos-size" => changed.general.pos_font_size = 28,
                "candidate-bold" => changed.general.candidate_bold = true,
                "annotation-bold" => changed.general.annotation_bold = true,
                "maximum" => {
                    changed.general.candidate_font_size = 48;
                    changed.general.annotation_font_size = 48;
                    changed.general.pos_font_size = 48;
                    changed.general.candidate_bold = true;
                    changed.general.annotation_bold = true;
                }
                "minimum" => {
                    changed.general.candidate_font_size = 8;
                    changed.general.annotation_font_size = 8;
                    changed.general.pos_font_size = 8;
                }
                "dark" => changed.general.theme = ThemeMode::Dark,
                "background-color" => changed.general.candidate_background_color = "#F4F1EA".into(),
                "candidate-color" => changed.general.candidate_text_color = "#243449".into(),
                "pos-color" => changed.general.candidate_pos_color = "#8C5E35".into(),
                "word-color" => changed.general.candidate_word_color = "#23635B".into(),
                "fresh-word-color" => changed.general.candidate_fresh_word_color = "#8844CC".into(),
                "highlight-color" => changed.general.candidate_highlight_color = "#D8E8D0".into(),
                _ => unreachable!(),
            }
            settings.sync(
                &changed,
                false,
                None,
                &[],
                &preferences::UpdateStatus::default(),
            );
            assert_ne!(
                png(&preview)?,
                original,
                "{backend}/{scenario} 没有更新预览"
            );
            let frame = preview.frame();
            // SAFETY: 设置窗口在本函数期间持有预览的父视图，且全程在主线程。
            let container = unsafe { preview.superview() }.unwrap().bounds();
            assert!(
                frame.origin.x + frame.size.width <= container.size.width,
                "{backend}/{scenario} 预览超宽"
            );
            assert!(
                frame.origin.y + frame.size.height <= container.size.height,
                "{backend}/{scenario} 预览超高"
            );
            save(
                &preview,
                &out.join(format!("settings-preview-{backend}-{scenario}.png")),
            )?;
            settings.sync(
                &baseline,
                false,
                None,
                &[],
                &preferences::UpdateStatus::default(),
            );
            assert_eq!(
                png(&preview)?,
                original,
                "{backend}/{scenario} 还原后显示不一致"
            );
        }
    }
    println!("两种渲染引擎的 32 组实时更新、还原与边界检查通过");
    Ok(())
}

fn sample_colors() -> GeneralConfig {
    GeneralConfig {
        candidate_background_color: "#F4F1EA".into(),
        candidate_text_color: "#243449".into(),
        candidate_pos_color: "#8C5E35".into(),
        candidate_word_color: "#23635B".into(),
        candidate_fresh_word_color: "#8844CC".into(),
        candidate_highlight_color: "#D8E8D0".into(),
        ..GeneralConfig::default()
    }
}

/// 从原生选色控件取值，经真实配置写入/读取后再刷新设置页，保留已有注释和字体。
fn check_color_controls(
    settings: &preferences::PreferencesWindow,
    content: &NSView,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = out.join("color-controls-test.toml");
    std::fs::write(
        &path,
        "# 保留这条注释\n[general]\ncandidate_font_size = 18\nfont = 'Helvetica'\n",
    )?;
    let preview = find_candidate_view(content).unwrap();
    let mut wells = Vec::new();
    collect_color_wells(content, &mut wells);
    assert_eq!(wells.len(), 6);
    for well in wells {
        for expected in ["#23635B", "#23635B80"] {
            let color = candidates::colors::parse_hex(expected).unwrap();
            well.setColor(&candidates::colors::native_color(color));
            let (setting, preferences::SettingValue::Text(value)) =
                preferences::setting_from_sender(Some(&well)).unwrap()
            else {
                panic!("原生选色控件没有返回色值")
            };
            assert_eq!(value, expected);
            Config::set_value(
                &path,
                "general",
                setting.color_key().unwrap(),
                value.as_str(),
            )?;
            let config = Config::load(&path)?;
            assert_eq!(config.general.candidate_font_size, 18);
            assert_eq!(config.general.font, "Helvetica");
            settings.sync(
                &config,
                false,
                None,
                &[],
                &preferences::UpdateStatus::default(),
            );
            let actual = preferences::setting_from_sender(Some(&well)).unwrap();
            assert!(
                matches!(actual.1, preferences::SettingValue::Text(ref value) if value == expected)
            );
            Config::set_value(&path, "general", setting.color_key().unwrap(), "")?;
            settings.sync(
                &Config::load(&path)?,
                false,
                None,
                &[],
                &preferences::UpdateStatus::default(),
            );
        }
    }
    assert!(std::fs::read_to_string(&path)?.contains("# 保留这条注释"));
    let mut config = Config {
        general: sample_colors(),
        ..Config::default()
    };
    config.general.candidate_font_size = 18;
    for (backend, renderer) in [
        ("qingjian", CandidateRenderer::Qingjian),
        ("system", CandidateRenderer::System),
    ] {
        config.general.renderer = renderer;
        // 加粗使字体库重建，确认自定义颜色不会在这一过程中丢失。
        config.general.candidate_bold = true;
        settings.sync(
            &config,
            false,
            None,
            &[],
            &preferences::UpdateStatus::default(),
        );
        save(
            &preview,
            &out.join(format!("settings-colors-{backend}.png")),
        )?;
    }
    println!("六个原生选色控件的 sRGB、透明度、持久化、逐项重置与注释保留通过");
    Ok(())
}

fn collect_color_wells(view: &NSView, result: &mut Vec<Retained<NSColorWell>>) {
    if let Some(well) = view.downcast_ref::<NSColorWell>() {
        result.push(well.retain());
    }
    for child in view.subviews() {
        collect_color_wells(&child, result);
    }
}

fn find_candidate_view(view: &NSView) -> Option<Retained<NSView>> {
    if view
        .class()
        .name()
        .to_str()
        .is_ok_and(|name| name.contains("::candidates::view::CandidateView"))
    {
        return Some(view.retain());
    }
    view.subviews()
        .iter()
        .find_map(|child| find_candidate_view(&child))
}

fn save(view: &NSView, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, png(view)?)?;
    let bounds = view.bounds();
    println!(
        "{}: {:.0} × {:.0} pt",
        path.display(),
        bounds.size.width,
        bounds.size.height
    );
    Ok(())
}

fn png(view: &NSView) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    view.displayIfNeeded();
    let bounds = view.bounds();
    let rep = view
        .bitmapImageRepForCachingDisplayInRect(bounds)
        .ok_or("无法缓存视图")?;
    view.cacheDisplayInRect_toBitmapImageRep(bounds, &rep);
    // SAFETY: 空字典使用 PNG 编码默认选项。
    let data = unsafe {
        rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
    .ok_or("PNG 编码失败")?;
    Ok(data.to_vec())
}
