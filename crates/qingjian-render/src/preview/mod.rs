//! 主题预览图：用样例帧把一个主题目录画成投稿用的截图，写进它的 `preview/`。
//! 两个壳的 `--theme-preview <主题目录>` 调它，主题仓库的 `tools/preview.py` 再调壳；规则见主题仓库 README「提交主题」。

mod error;
mod shot;

use std::path::{Path, PathBuf};

pub use error::{PreviewError, ShotError};
pub use shot::Shot;

use crate::sample::{nihao, nihao_with_sentence};
use crate::{FontLibrary, Frame, Layout, Renderer, Theme};

/// 2 倍图：与 Retina 截屏一样清楚，单张在主题仓库的 500 KB 上限以内。
const SCALE: f32 = 2.0;

/// 命令行 `--theme-preview` 的命令名。
pub const FLAG: &str = "--theme-preview";

/// 壳的命令行入口：把 `theme_dir` 的预览图写进 `theme_dir/preview/`，逐张打印结果，返回进程退出码。
/// 至少写出一张算成功（缺的那张单独列出来，不让整个命令失败）；主题读不进来或一张都没写出是 1。
pub fn run(theme_dir: &Path, family_files: impl FnMut(&str) -> Vec<PathBuf>) -> i32 {
    let shots = match write_previews(theme_dir, &theme_dir.join("preview"), family_files) {
        Ok(shots) => shots,
        Err(error) => {
            eprintln!("无法生成预览图：{error}");
            return 1;
        }
    };
    let mut missing = Vec::new();
    for shot in &shots {
        match &shot.result {
            Ok(path) => println!("已写出 {}", path.display()),
            Err(error) => {
                eprintln!("{} 生成失败：{error}", shot.file);
                missing.push(shot.file);
            }
        }
    }
    if missing.len() == shots.len() {
        return 1;
    }
    if !missing.is_empty() {
        eprintln!("缺 {}，其余已写出", missing.join("、"));
    }
    0
}

/// 画 `theme_dir` 的竖排、横排截图（主题没锁定外观时再加一张深色竖排），写到 `out_dir`，同名文件覆盖。
/// 某一张画坏了记在它的 [`Shot`] 里、其余照画；主题读不进来或字体库建不起来才整体失败。
/// `family_files` 是壳按字族名找系统字体文件的函数，与候选窗加载主题字体时同一个。
pub fn write_previews(
    theme_dir: &Path,
    out_dir: &Path,
    family_files: impl FnMut(&str) -> Vec<PathBuf>,
) -> Result<Vec<Shot>, PreviewError> {
    let theme = Theme::from_dir(theme_dir, false)?;
    let mut renderer = Renderer::new(FontLibrary::system("zh-CN")?);
    renderer.load_theme_fonts(&theme, family_files);
    std::fs::create_dir_all(out_dir).map_err(|source| PreviewError::OutDir {
        path: out_dir.to_owned(),
        source,
    })?;
    let mut shots = vec![
        ("vertical.png", nihao(), Layout::Vertical, false),
        (
            "horizontal.png",
            nihao_with_sentence(),
            Layout::Horizontal,
            false,
        ),
    ];
    if theme.locked_dark().is_none() {
        shots.push(("vertical-dark.png", nihao(), Layout::Vertical, true));
    }
    Ok(shots
        .into_iter()
        .map(|(file, frame, layout, dark)| Shot {
            file,
            result: draw(
                &mut renderer,
                &theme.with_dark(dark),
                &frame,
                layout,
                &out_dir.join(file),
            ),
        })
        .collect())
}

fn draw(
    renderer: &mut Renderer,
    theme: &Theme,
    frame: &Frame,
    layout: Layout,
    path: &Path,
) -> Result<PathBuf, ShotError> {
    // 每张独立，不和上一张配对播过渡
    renderer.forget();
    let rendered = renderer.render(frame, layout, theme, SCALE)?;
    rendered
        .pixmap
        .save_png(path)
        .map_err(|source| ShotError::Save {
            path: path.to_owned(),
            message: source.to_string(),
        })?;
    Ok(path.to_owned())
}
