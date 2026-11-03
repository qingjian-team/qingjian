//! [`Edge`]：一条贴边窗口的形状，以及它每帧的像素（平台无关，窗口与贴图在各平台的 `neon_strip`）。

use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// 像素的通道顺序；都是预乘过透明度的 8 位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PixelOrder {
    /// Windows 的 DIB。
    #[cfg_attr(not(windows), allow(dead_code))]
    Bgra,

    /// macOS 的 `NSBitmapImageRep`。
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Rgba,
}

/// 一条贴边窗口：在显示器哪条边、多大，显示器多大（都是同一种单位，像素或点）。
#[derive(Debug, Clone, Copy)]
pub(crate) struct Edge {
    pub side: Side,

    pub size: (i32, i32),

    pub monitor: (i32, i32),
}

impl Edge {
    /// 一台显示器四条边，厚度按显示器短边算。
    pub(crate) fn around(monitor: (i32, i32), thickness: (i32, i32)) -> [Self; 4] {
        let (width, height) = monitor;
        let thickness = (width.min(height) / 40).clamp(thickness.0, thickness.1);
        [
            (Side::Top, (width, thickness)),
            (Side::Bottom, (width, thickness)),
            (Side::Left, (thickness, height)),
            (Side::Right, (thickness, height)),
        ]
        .map(|(side, size)| Self {
            side,
            size,
            monitor,
        })
    }

    /// 这条边的左上角相对显示器左上角的位置（y 朝下）。
    pub(crate) fn offset(&self) -> (i32, i32) {
        let (width, height) = self.monitor;
        match self.side {
            Side::Top | Side::Left => (0, 0),
            Side::Bottom => (0, height - self.size.1),
            Side::Right => (width - self.size.0, 0),
        }
    }

    /// 像素沿屏幕一圈的位置（0..1）和离屏幕边的距离。
    fn locate(&self, x: i32, y: i32) -> (f32, i32) {
        let (width, height) = self.monitor;
        let perimeter = (2 * (width + height)) as f32;
        let (along, depth) = match self.side {
            Side::Top => (x, y),
            Side::Right => (width + y, self.size.0 - 1 - x),
            Side::Bottom => (width + height + (width - x), self.size.1 - 1 - y),
            Side::Left => (2 * width + height + (height - y), x),
        };
        (along as f32 / perimeter, depth)
    }

    /// 画一帧到 `pixels`（宽 × 高 × 4，从上到下）。
    pub(crate) fn paint(
        &self,
        pixels: &mut [u8],
        order: PixelOrder,
        color: [f32; 3],
        time: f32,
        strength: f32,
    ) {
        let (columns, rows) = self.size;
        let thickness = columns.min(rows) as f32;
        for y in 0..rows {
            for x in 0..columns {
                let (phase, depth) = self.locate(x, y);
                let fall = (1.0 - depth as f32 / thickness).max(0.0);
                let wave = 0.62 + 0.38 * ((phase * 3.0 - time * 0.7) * TAU).cos();
                let alpha = fall * fall * wave * strength;
                let shade = color;
                let offset = ((y * columns + x) * 4) as usize;
                let Some(pixel) = pixels.get_mut(offset..offset + 4) else {
                    return;
                };
                let (first, last) = match order {
                    PixelOrder::Bgra => (shade[2], shade[0]),
                    PixelOrder::Rgba => (shade[0], shade[2]),
                };
                pixel[0] = (first * alpha) as u8;
                pixel[1] = (shade[1] * alpha) as u8;
                pixel[2] = (last * alpha) as u8;
                pixel[3] = (alpha * 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Edge, PixelOrder, Side};

    #[test]
    fn channel_order_swaps_red_and_blue() {
        let edge = Edge {
            side: Side::Left,
            size: (4, 4),
            monitor: (100, 100),
        };
        let mut bgra = vec![0_u8; 64];
        let mut rgba = vec![0_u8; 64];
        let red = [255.0, 0.0, 0.0];
        edge.paint(&mut bgra, PixelOrder::Bgra, red, 0.0, 1.0);
        edge.paint(&mut rgba, PixelOrder::Rgba, red, 0.0, 1.0);
        assert!(rgba[0] > 0 && rgba[2] == 0);
        assert_eq!((bgra[0], bgra[2]), (rgba[2], rgba[0]));
    }

    #[test]
    fn edges_sit_on_the_four_sides() {
        let edges = Edge::around((1440, 900), (16, 40));
        assert_eq!(edges[0].offset(), (0, 0));
        assert_eq!(edges[1].offset(), (0, 900 - 22));
        assert_eq!(edges[3].offset(), (1440 - 22, 0));
        assert_eq!(edges[2].size, (22, 900));
    }
}
