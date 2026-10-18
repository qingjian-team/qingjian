//! AAT `trak` 字距表：Apple 的系统字体按字号给每个字形加减一点间距（SF 在 11 pt 是 +12、16 pt 是 −40 个字体单位），
//! CoreText 对系统字体自动应用；不读它，英文比原生窄或宽一截。只取水平方向、正常轨（track 0）。

/// 一张字体的水平字距曲线。
#[derive(Debug, Clone, PartialEq)]
pub struct Trak {
    /// 采样字号（点），升序。
    sizes: Vec<f32>,

    /// 每个采样字号的间距（字体单位），与 `sizes` 一一对应。
    values: Vec<i16>,

    /// 字体单位数。
    units_per_em: f32,
}

impl Trak {
    /// 从一张 sfnt 面的数据里读；没有 `trak` 表或没有正常轨返回 `None`。
    pub fn parse(face_data: &[u8], face_index: u32) -> Option<Self> {
        let units_per_em = f32::from(u16_at(table(face_data, face_index, b"head")?, 18)?);
        let trak = table(face_data, face_index, b"trak")?;
        let horiz_offset = u16_at(trak, 6)? as usize;
        let data = trak.get(horiz_offset..)?;
        let n_tracks = u16_at(data, 0)? as usize;
        let n_sizes = u16_at(data, 2)? as usize;
        let size_table_offset = u32_at(data, 4)? as usize;
        let sizes: Vec<f32> = (0..n_sizes)
            .map(|i| u32_at(trak, size_table_offset + i * 4).map(fixed_to_f32))
            .collect::<Option<_>>()?;
        // 轨表每条 8 字节：track（Fixed）、名字下标、到 n_sizes 个 i16 的偏移
        let normal = (0..n_tracks)
            .map(|i| 8 + i * 8)
            .find(|&entry| u32_at(data, entry).map(fixed_to_f32) == Some(0.0))?;
        let values_offset = u16_at(data, normal + 6)? as usize;
        let values: Vec<i16> = (0..n_sizes)
            .map(|i| u16_at(trak, values_offset + i * 2).map(|v| v as i16))
            .collect::<Option<_>>()?;
        Some(Self {
            sizes,
            values,
            units_per_em,
        })
    }

    /// 某字号下每个字形要加的间距，单位 em（乘字号就是点）。采样点之间线性插值，范围外取端点。
    pub fn tracking_em(&self, size: f32) -> f32 {
        let units = match self.sizes.iter().position(|&s| s >= size) {
            Some(0) => f32::from(self.values[0]),
            None => f32::from(*self.values.last().unwrap_or(&0)),
            Some(i) => {
                let (s0, s1) = (self.sizes[i - 1], self.sizes[i]);
                let (v0, v1) = (f32::from(self.values[i - 1]), f32::from(self.values[i]));
                // 采样点该严格升序；相邻相等的坏表别除出 NaN。
                if s1 <= s0 {
                    v0
                } else {
                    v0 + (v1 - v0) * (size - s0) / (s1 - s0)
                }
            }
        };
        units / self.units_per_em
    }
}

/// sfnt 表目录里找一张表；TTC 先按面下标取那张目录。
fn table<'a>(data: &'a [u8], face_index: u32, tag: &[u8; 4]) -> Option<&'a [u8]> {
    let directory = if data.get(0..4)? == b"ttcf" {
        u32_at(data, 12 + face_index as usize * 4)? as usize
    } else {
        0
    };
    let count = u16_at(data, directory + 4)? as usize;
    (0..count)
        .map(|i| directory + 12 + i * 16)
        .find_map(|entry| {
            (data.get(entry..entry + 4)? == tag).then(|| {
                let offset = u32_at(data, entry + 8)? as usize;
                let length = u32_at(data, entry + 12)? as usize;
                data.get(offset..offset + length)
            })?
        })
}

fn u16_at(data: &[u8], offset: usize) -> Option<u16> {
    data.get(offset..offset + 2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
}

fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn fixed_to_f32(fixed: u32) -> f32 {
    fixed as i32 as f32 / 65536.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_between_sizes_and_clamps() {
        let trak = Trak {
            sizes: vec![11.0, 12.0, 16.0],
            values: vec![12, 0, -40],
            units_per_em: 2048.0,
        };
        assert!((trak.tracking_em(11.0) * 2048.0 - 12.0).abs() < 1e-3);
        assert!((trak.tracking_em(14.0) * 2048.0 + 20.0).abs() < 1e-3);
        assert!((trak.tracking_em(8.0) * 2048.0 - 12.0).abs() < 1e-3);
        assert!((trak.tracking_em(40.0) * 2048.0 + 40.0).abs() < 1e-3);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reads_sf_tracking_table() {
        let data = std::fs::read("/System/Library/Fonts/SFNS.ttf").unwrap();
        let trak = Trak::parse(&data, 0).unwrap();
        assert_eq!(trak.units_per_em, 2048.0);
        assert!((trak.tracking_em(12.0)).abs() < 1e-6);
        assert!((trak.tracking_em(16.0) * 2048.0 + 40.0).abs() < 1e-3);
    }
}
