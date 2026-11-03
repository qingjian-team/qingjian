//! 把托盘图标的 SVG 栅格化成 `resources/tray-{dark,light}.ico`：16–48 存 BMP，64 起存 PNG，与输入法的图标同构。
//! 改了 `assets/icon/windows/clipboard-tray*.svg` 后在仓库根跑：`cargo run -p qingjian-clipboard-app --example render_tray_icon`。

use std::error::Error;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

const SOURCE: &str = "assets/icon/windows/clipboard-tray.svg";

const SOURCE_16: &str = "assets/icon/windows/clipboard-tray-16.svg";

/// 源文件的墨色，也是深色任务栏版的墨色。
const WHITE_INK: &str = "#FFFFFF";

const DARK_INK: &str = "#1F1F1F";

const SIZES: [u32; 8] = [16, 20, 24, 32, 48, 64, 128, 256];

/// 这一档起存 PNG：老的读取方只认 BMP，小尺寸留给它们。
const FIRST_PNG_SIZE: u32 = 64;

fn render(svg: &str, ink: &str, size: u32) -> Result<Pixmap, Box<dyn Error>> {
    let tree = usvg::Tree::from_str(&svg.replace(WHITE_INK, ink), &usvg::Options::default())?;
    let mut pixmap = Pixmap::new(size, size).ok_or("empty icon size")?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

/// ICO 里的 BMP：高度写两倍，从下往上存 BGRA，后面跟 1 位的透明蒙版。
fn bmp_entry(pixmap: &Pixmap) -> Vec<u8> {
    let size = pixmap.width() as usize;
    let mut out = Vec::new();
    out.extend(40_u32.to_le_bytes());
    out.extend((size as i32).to_le_bytes());
    out.extend((size as i32 * 2).to_le_bytes());
    out.extend(1_u16.to_le_bytes());
    out.extend(32_u16.to_le_bytes());
    out.extend([0_u8; 24]);
    let rows: Vec<_> = pixmap.pixels().chunks(size).rev().collect();
    for row in &rows {
        for pixel in *row {
            let color = pixel.demultiply();
            out.extend([color.blue(), color.green(), color.red(), color.alpha()]);
        }
    }
    let mask_row = size.div_ceil(32) * 4;
    for row in &rows {
        let mut mask = vec![0_u8; mask_row];
        for (x, pixel) in row.iter().enumerate() {
            if pixel.alpha() == 0 {
                mask[x / 8] |= 0x80 >> (x % 8);
            }
        }
        out.extend(mask);
    }
    out
}

fn icon(general: &str, small: &str, ink: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut entries = Vec::new();
    for size in SIZES {
        let pixmap = render(if size == 16 { small } else { general }, ink, size)?;
        let data = if size < FIRST_PNG_SIZE {
            bmp_entry(&pixmap)
        } else {
            pixmap.encode_png()?
        };
        entries.push((size, data));
    }
    let mut header = vec![0, 0, 1, 0];
    header.extend((entries.len() as u16).to_le_bytes());
    let mut offset = (6 + 16 * entries.len()) as u32;
    let mut body = Vec::new();
    for (size, data) in entries {
        header.extend([size as u8, size as u8, 0, 0]);
        header.extend(1_u16.to_le_bytes());
        header.extend(32_u16.to_le_bytes());
        header.extend((data.len() as u32).to_le_bytes());
        header.extend(offset.to_le_bytes());
        offset += data.len() as u32;
        body.extend(data);
    }
    header.extend(body);
    Ok(header)
}

fn main() -> Result<(), Box<dyn Error>> {
    let general = std::fs::read_to_string(SOURCE)?;
    let small = std::fs::read_to_string(SOURCE_16)?;
    for (name, ink) in [("dark", WHITE_INK), ("light", DARK_INK)] {
        let path = format!("apps/clipboard/resources/tray-{name}.ico");
        std::fs::write(path, icon(&general, &small, ink)?)?;
    }
    Ok(())
}
