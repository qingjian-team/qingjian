//! 阴影用的模糊：三遍盒式模糊近似高斯，作用在一张 alpha 遮罩上。

/// 模糊 `mask`（`width × height` 的 alpha），`blur` 是像素，与主题里的 `blur` 同义；边界外当 0。
pub(crate) fn blur(mask: &mut [u8], width: usize, height: usize, blur: f32) {
    let radius = (blur / 2.0).round().max(1.0) as usize;
    for _ in 0..3 {
        box_blur_horizontal(mask, width, height, radius);
        box_blur_vertical(mask, width, height, radius);
    }
}

/// 模糊在形状四周能铺开多远（像素）。
pub(crate) fn reach(blur: f32) -> f32 {
    blur * 2.0
}

fn box_blur_horizontal(data: &mut [u8], width: usize, height: usize, radius: usize) {
    let mut row_buf = vec![0u8; width];
    for row in 0..height {
        let line = &data[row * width..(row + 1) * width];
        box_blur_line(line, &mut row_buf, radius);
        data[row * width..(row + 1) * width].copy_from_slice(&row_buf);
    }
}

fn box_blur_vertical(data: &mut [u8], width: usize, height: usize, radius: usize) {
    let mut column = vec![0u8; height];
    let mut blurred = vec![0u8; height];
    for col in 0..width {
        for row in 0..height {
            column[row] = data[row * width + col];
        }
        box_blur_line(&column, &mut blurred, radius);
        for row in 0..height {
            data[row * width + col] = blurred[row];
        }
    }
}

/// 一维盒式模糊，窗口 `2r + 1`，边界外当 0；滑动窗口 O(n)。
fn box_blur_line(src: &[u8], dst: &mut [u8], radius: usize) {
    let n = src.len();
    let window = (2 * radius + 1) as u32;
    let mut sum: u32 = src.iter().take(radius + 1).map(|&v| u32::from(v)).sum();
    for i in 0..n {
        dst[i] = ((sum + window / 2) / window) as u8;
        if i + radius + 1 < n {
            sum += u32::from(src[i + radius + 1]);
        }
        if i >= radius {
            sum -= u32::from(src[i - radius]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_spreads_and_preserves_mass() {
        let src = [0, 0, 255, 0, 0];
        let mut dst = [0u8; 5];
        box_blur_line(&src, &mut dst, 1);
        assert_eq!(dst, [0, 85, 85, 85, 0]);
    }
}
