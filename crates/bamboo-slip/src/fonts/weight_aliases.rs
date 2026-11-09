//! 补齐字族缺的字重。cosmic-text 挑字体只认字重完全相等的面（可变字体除外）：微软雅黑只有 290 / 400 / 700，
//! 要 500 / 600 时整个字族被筛掉，汉字逐字落到恰好有这档字重的 Yu Gothic，一句话里中日字形、大小混排。
//! 这里给每个字族缺的整百字重补登记一张面，指向按 CSS 字重匹配规则挑出的真实面；数据与原面共用同一份文件映射，不另读。

use std::collections::HashMap;

use cosmic_text::fontdb::{Database, ID, Stretch, Style, Weight};

/// 补登记的面。字体库加载了新文件就整批删掉重补，免得旧的补位挡住新来的真实字重。
#[derive(Default)]
pub(crate) struct WeightAliases {
    ids: Vec<ID>,
}

impl WeightAliases {
    pub(crate) fn refresh(&mut self, db: &mut Database) {
        for id in self.ids.drain(..) {
            db.remove_face(id);
        }
        let mut groups: HashMap<(String, Style, Stretch), Vec<(u16, ID)>> = HashMap::new();
        for face in db.faces() {
            let Some((family, _)) = face.families.first() else {
                continue;
            };
            groups
                .entry((family.clone(), face.style, face.stretch))
                .or_default()
                .push((face.weight.0, face.id));
        }
        for faces in groups.values() {
            let weights: Vec<u16> = faces.iter().map(|&(weight, _)| weight).collect();
            for target in (100..=900).step_by(100) {
                if weights.contains(&target) {
                    continue;
                }
                let Some(nearest) = nearest_weight(&weights, target) else {
                    continue;
                };
                let Some(&(_, id)) = faces.iter().find(|&&(weight, _)| weight == nearest) else {
                    continue;
                };
                let Some(mut info) = db.face(id).cloned() else {
                    continue;
                };
                info.weight = Weight(target);
                self.ids.push(db.push_face_info(info));
            }
        }
    }
}

/// CSS 的字重匹配：要 400～500 先往上找到 500，再往下、再往 500 以上；要更细的先往下；要更粗的先往上。
fn nearest_weight(weights: &[u16], target: u16) -> Option<u16> {
    let below = || weights.iter().copied().filter(|&w| w < target).max();
    let above = |from: u16| weights.iter().copied().filter(|&w| w > from).min();
    if (400..=500).contains(&target) {
        weights
            .iter()
            .copied()
            .filter(|&w| w > target && w <= 500)
            .min()
            .or_else(below)
            .or_else(|| above(500))
    } else if target < 400 {
        below().or_else(|| above(target))
    } else {
        above(target).or_else(below)
    }
}

#[cfg(test)]
mod tests {
    use super::nearest_weight;

    #[test]
    fn follows_css_weight_matching() {
        let yahei = [290, 400, 700];
        assert_eq!(nearest_weight(&yahei, 300), Some(290));
        assert_eq!(nearest_weight(&yahei, 500), Some(400));
        assert_eq!(nearest_weight(&yahei, 600), Some(700));
        assert_eq!(nearest_weight(&yahei, 100), Some(290));
        assert_eq!(nearest_weight(&yahei, 900), Some(700));

        let segoe = [300, 350, 400, 600, 700, 900];
        assert_eq!(nearest_weight(&segoe, 500), Some(400));
        assert_eq!(nearest_weight(&segoe, 800), Some(900));
        assert_eq!(nearest_weight(&[400, 500], 450), Some(500));
        assert_eq!(nearest_weight(&[], 400), None);
    }

    /// 简体界面里 300 / 500 / 600 的汉字都用雅黑，不逐字混进 Yu Gothic。
    #[cfg(target_os = "windows")]
    #[test]
    fn han_keeps_yahei_at_every_weight() {
        use crate::color::Color;
        use crate::fonts::FontLibrary;
        use crate::text::font::{FontSpec, FontWeight};
        use crate::text::{TextPainter, TextStyle};

        let mut text = TextPainter::new(FontLibrary::system("zh-CN").unwrap());
        for weight in [300, 400, 500, 600, 700] {
            let mut style =
                TextStyle::new(FontSpec::new(16.0, 20.0), 16.0, Color::rgb(0, 0, 0), 1.0);
            style.weight = FontWeight(weight);
            let families = text.trace_families("生词本自定义短语", &style);
            assert!(
                families
                    .iter()
                    .all(|name| name.starts_with("Microsoft YaHei")),
                "{weight}: {families:?}"
            );
        }
    }
}
