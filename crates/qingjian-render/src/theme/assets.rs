//! 主题的图片与字体素材：加载主题时读进来，图片解码一次、渲染时按路径取；字体只核对路径，由渲染器加载。
//! 用户主题从主题目录读；内置主题的图片编进程序（`include_bytes!`），按同样的路径查表，不带字体（只用系统字体）。
//!
//! 路径相对 `theme.json`，不能是绝对路径、不能含 `..`（主题不能读主题目录以外的文件）；图片认 PNG（边长上限 [`MAX_SIDE`]）
//! 与 SVG（`.svg` 结尾，文件上限 [`MAX_SVG_BYTES`]、原始尺寸不超过 [`MAX_SIDE`]）；
//! 字体单个文件上限 [`MAX_FONT_BYTES`]。读不进来的记警告，用到图片的填充不画，用到字体的样式按回退链往后找。

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use bamboo_slip::SvgImage;
use bamboo_slip::tiny_skia::Pixmap;

use super::file::ThemeFile;
use super::file::node::{FillSpec, NodeKind, NodeSpec};

/// 图片边长上限（像素）。
const MAX_SIDE: u32 = 4096;

/// 单个 SVG 文件上限（字节）。
const MAX_SVG_BYTES: u64 = 4 * 1024 * 1024;

/// 单个 PNG 文件上限（字节）：没压缩的 4096 × 4096 × 4 也就 64 MB，压缩后只会更小。
const MAX_PNG_BYTES: u64 = (MAX_SIDE as u64) * (MAX_SIDE as u64) * 4;

/// PNG 的签名。
const PNG_SIGNATURE: [u8; 8] = *b"\x89PNG\r\n\x1a\n";

/// 单个主题最多加载多少张图片：正常主题十几张，多到这个数就是在凑数。
const MAX_IMAGES: usize = 64;

/// 单个主题最多带多少份字体。
const MAX_FONTS: usize = 16;

/// 随主题带的单个字体文件上限（字节）：裁过字的中文字体在 10 MB 以内，整个主题包上限 32 MB。
const MAX_FONT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct Assets {
    /// 主题里写的路径 → 解码好的预乘位图。
    images: HashMap<String, Arc<Pixmap>>,

    /// 主题里写的路径 → 解析好的 SVG。
    svgs: HashMap<String, Arc<SvgImage>>,

    /// 随主题带的字体文件。
    fonts: Vec<PathBuf>,
}

impl Assets {
    /// 读 `file` 里用到的全部图片与字体；`dir` 是主题目录。`inherited` 是所继承内置主题编进程序的图片：
    /// 主题目录里没有的图片到那里找，`extends` 樱花这类带图的内置主题、只改颜色字号时不用把图拷一份。
    pub(crate) fn load(file: &ThemeFile, dir: &Path, inherited: &[(&str, &'static [u8])]) -> Self {
        let mut assets = Self::decode(file, |path, limit| match read_file(dir, path, limit) {
            Err(MISSING) => find_embedded(inherited, path),
            read => read,
        });
        for font in &file.fonts {
            if assets.fonts.len() >= MAX_FONTS {
                tracing::warn!(id = file.meta.id, "主题字体太多，后面的不用");
                break;
            }
            match font_path(dir, &font.file) {
                Ok(path) if !assets.fonts.contains(&path) => assets.fonts.push(path),
                Ok(_) => {}
                Err(problem) => {
                    tracing::warn!(
                        id = file.meta.id,
                        path = font.file,
                        "主题字体{problem}，不用"
                    );
                }
            }
        }
        assets
    }

    /// 内置主题：图片从编进程序的文件表（路径 → 内容）里取。
    pub(crate) fn embedded(file: &ThemeFile, files: &[(&str, &'static [u8])]) -> Self {
        Self::decode(file, |path, _| find_embedded(files, path))
    }

    /// 解码节点树里用到的全部图片；`read` 按路径给文件内容，第二个参数是这种图片的字节上限。
    fn decode<'a>(
        file: &ThemeFile,
        read: impl Fn(&str, u64) -> Result<Cow<'a, [u8]>, &'static str>,
    ) -> Self {
        let mut images = HashMap::new();
        let mut svgs = HashMap::new();
        for path in image_paths(file) {
            if images.contains_key(&path) || svgs.contains_key(&path) {
                continue;
            }
            if images.len() + svgs.len() >= MAX_IMAGES {
                tracing::warn!(id = file.meta.id, path, "主题图片太多，后面的不读");
                break;
            }
            let loaded = if is_svg(&path) {
                read(&path, MAX_SVG_BYTES)
                    .and_then(|data| decode_svg(&data))
                    .map(|svg| {
                        svgs.insert(path.clone(), Arc::new(svg));
                    })
            } else {
                read(&path, MAX_PNG_BYTES)
                    .and_then(|data| decode_png(&data))
                    .map(|image| {
                        images.insert(path.clone(), Arc::new(image));
                    })
            };
            if let Err(problem) = loaded {
                tracing::warn!(id = file.meta.id, path, "主题图片{problem}，不画");
            }
        }
        Self {
            images,
            svgs,
            fonts: Vec::new(),
        }
    }

    pub(crate) fn image(&self, path: &str) -> Option<Arc<Pixmap>> {
        self.images.get(path).cloned()
    }

    pub(crate) fn svg(&self, path: &str) -> Option<Arc<SvgImage>> {
        self.svgs.get(path).cloned()
    }

    pub(crate) fn fonts(&self) -> &[PathBuf] {
        &self.fonts
    }
}

/// 主题里写的相对路径 → 主题目录里的路径；绝对路径与 `..` 拒掉。
fn inside(dir: &Path, path: &str) -> Result<PathBuf, &'static str> {
    let relative = Path::new(path);
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err("路径不在主题目录里");
    }
    Ok(dir.join(relative))
}

fn font_path(dir: &Path, path: &str) -> Result<PathBuf, &'static str> {
    let path = inside(dir, path)?;
    let size = std::fs::metadata(&path).map_err(|_| "不存在")?.len();
    if size > MAX_FONT_BYTES {
        return Err("太大");
    }
    Ok(path)
}

fn is_svg(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
}

/// 文件不存在时的问题描述（目录里没有就去继承的内置主题里找）。
const MISSING: &str = "不存在";

fn find_embedded(
    files: &[(&str, &'static [u8])],
    path: &str,
) -> Result<Cow<'static, [u8]>, &'static str> {
    files
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, data)| Cow::Borrowed(*data))
        .ok_or(MISSING)
}

/// 主题目录里的文件，超过 `limit` 字节的不读。
fn read_file(dir: &Path, path: &str, limit: u64) -> Result<Cow<'static, [u8]>, &'static str> {
    let path = inside(dir, path)?;
    let size = std::fs::metadata(&path).map_err(|_| MISSING)?.len();
    if size > limit {
        return Err("太大");
    }
    let data = std::fs::read(&path).map_err(|_| "读不进来")?;
    // stat 与读之间文件可能被换掉，按实际读到的长度再确认
    if data.len() as u64 > limit {
        return Err("太大");
    }
    Ok(Cow::Owned(data))
}

fn decode_svg(data: &[u8]) -> Result<SvgImage, &'static str> {
    let svg = SvgImage::parse(data).map_err(|_| "不是能解析的 SVG")?;
    let (width, height) = svg.size();
    if width > MAX_SIDE as f32 || height > MAX_SIDE as f32 {
        return Err("太大");
    }
    Ok(svg)
}

/// PNG 的 IHDR 里声明的宽高；签名或 IHDR 读不出来为 `None`。
fn png_size(data: &[u8]) -> Option<(u32, u32)> {
    let ihdr = data.get(12..24)?;
    if !ihdr.starts_with(b"IHDR") {
        return None;
    }
    Some((
        u32::from_be_bytes([ihdr[4], ihdr[5], ihdr[6], ihdr[7]]),
        u32::from_be_bytes([ihdr[8], ihdr[9], ihdr[10], ihdr[11]]),
    ))
}

fn decode_png(data: &[u8]) -> Result<Pixmap, &'static str> {
    if !data.starts_with(&PNG_SIGNATURE) {
        return Err("读不进来（只认 PNG）");
    }
    // 解码器按 IHDR 声明的尺寸先分配再解压：一个二十来字节、声称 100000 × 100000 的文件
    // 会让它直接要四十 GB。尺寸必须在交出去之前查。
    match png_size(data) {
        Some((width, height)) if width > 0 && height > 0 && width <= MAX_SIDE && height <= MAX_SIDE => {}
        _ => return Err("太大"),
    }
    Pixmap::decode_png(data).map_err(|_| "读不进来（只认 PNG）")
}

/// 主题里写到的全部图片路径（可能重复）。
pub(super) fn image_paths(file: &ThemeFile) -> Vec<String> {
    let mut paths = Vec::new();
    for component in file.components.values() {
        collect(component, &mut paths);
    }
    collect(&file.windows.vertical, &mut paths);
    collect(&file.windows.horizontal, &mut paths);
    if let Some(root) = &file.status.root {
        collect(root, &mut paths);
    }
    let panel = &file.panel;
    for fill in [&panel.background, &panel.pill_fill, &panel.highlight] {
        if let FillSpec::Image { image, .. } = fill {
            paths.push(image.clone());
        }
    }
    paths
}

/// 收集节点树里的图片路径。
fn collect(node: &NodeSpec, out: &mut Vec<String>) {
    if let NodeKind::Frame { fill, children, .. } = &node.kind {
        if let Some(FillSpec::Image { image, .. }) = fill {
            out.push(image.clone());
        }
        for child in children {
            collect(child, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Assets, decode_png};
    use crate::theme::Theme;

    const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"/>"#;

    /// 二十来个字节就声称 100000 × 100000 的 PNG 开头。
    fn oversized_png_header() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&100_000u32.to_be_bytes());
        bytes.extend_from_slice(&100_000u32.to_be_bytes());
        bytes
    }

    #[test]
    fn a_png_that_claims_a_huge_size_is_rejected_before_decoding() {
        // 解码器会按声明先分配：这里必须连解码都不进
        assert_eq!(decode_png(&oversized_png_header()).unwrap_err(), "太大");
        let mut broken = oversized_png_header();
        broken[12..16].copy_from_slice(b"XXXX");
        assert_eq!(decode_png(&broken).unwrap_err(), "太大");
        assert_eq!(decode_png(b"hello").unwrap_err(), "读不进来（只认 PNG）");
    }

    #[test]
    fn embedded_files_are_found_by_theme_path() {
        let theme = Theme::from_json(
            r#"{ "extends": "qingjian", "schema": 1, "meta": { "id": "e", "name": "e" },
                 "windows": { "vertical": { "fill": { "image": "images/a.svg" } },
                              "horizontal": { "fill": { "image": "images/missing.png" } } } }"#,
            false,
        )
        .unwrap();
        let assets = Assets::embedded(theme.file(), &[("images/a.svg", SVG)]);
        assert!(assets.svg("images/a.svg").is_some());
        assert!(assets.image("images/missing.png").is_none());
        assert!(assets.fonts().is_empty());
    }
}
