//! 整张字根图：三排键，与键盘一样第二、三排错开，第三排从 Z 起（Z 是万能键，不放字根）。

use std::sync::OnceLock;

use super::{RootGlyph, RootKey};

/// 字形表：键、区位、键名字、字、说明、露出区域、SVG 路径。
const TABLE: &str = include_str!("../../../../assets/schemes/wubi86/roots86-glyphs.tsv");

const ROWS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];

#[derive(Debug, Clone, PartialEq)]
pub struct RootChart {
    pub rows: Vec<Vec<RootKey>>,
}

impl RootChart {
    /// 86 版字根图；第一次用到时解析，之后共用。
    pub fn wubi86() -> &'static Self {
        static CHART: OnceLock<RootChart> = OnceLock::new();
        CHART.get_or_init(|| Self::parse(TABLE))
    }

    fn parse(table: &'static str) -> Self {
        let mut keys: Vec<RootKey> = Vec::new();
        for line in table.lines().filter(|line| !line.starts_with('#')) {
            let fields: Vec<&'static str> = line.split('\t').collect();
            let [key, zone, _, source, _, frame, svg] = fields[..] else {
                continue;
            };
            let Some(key) = key.chars().next() else {
                continue;
            };
            let frame: Vec<f32> = frame
                .split_whitespace()
                .filter_map(|v| v.parse().ok())
                .collect();
            let glyph = RootGlyph {
                source,
                svg,
                frame: match frame[..] {
                    [left, top, right, bottom] => Some([left, top, right, bottom]),
                    _ => None,
                },
            };
            match keys.iter_mut().find(|k| k.key == key) {
                Some(found) => found.roots.push(glyph),
                None => keys.push(RootKey {
                    key,
                    zone: zone.parse().ok(),
                    roots: vec![glyph],
                }),
            }
        }
        let rows = ROWS
            .iter()
            .map(|row| {
                row.chars()
                    .map(|key| {
                        keys.iter()
                            .find(|k| k.key == key)
                            .cloned()
                            .unwrap_or(RootKey {
                                key,
                                zone: None,
                                roots: Vec::new(),
                            })
                    })
                    .collect()
            })
            .collect();
        Self { rows }
    }
}

#[cfg(test)]
mod tests {
    use super::RootChart;

    #[test]
    fn every_key_but_z_has_roots() {
        let chart = RootChart::wubi86();
        let keys: Vec<_> = chart.rows.iter().flatten().collect();
        assert_eq!(keys.len(), 26);
        for key in keys {
            assert_eq!(key.roots.is_empty(), key.key == 'Z', "{}", key.key);
            assert_eq!(key.zone.is_none(), key.key == 'Z', "{}", key.key);
        }
    }

    #[test]
    fn parts_carry_a_frame() {
        let chart = RootChart::wubi86();
        let tiger = chart.rows[1]
            .iter()
            .find(|k| k.key == 'H')
            .and_then(|k| k.roots.iter().find(|r| r.source == "虎"));
        assert!(tiger.is_some_and(|r| r.frame.is_some() && r.svg.starts_with('M')));
    }
}
