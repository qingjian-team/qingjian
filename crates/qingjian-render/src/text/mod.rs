//! 文字测绘：cosmic-text 整形 + swash 栅格，单行、像素坐标；普通字形走覆盖率遮罩，彩色 emoji 走 RGBA 位图。

mod size;
mod style;

use std::collections::HashMap;

use cosmic_text::fontdb::{Family, ID, Query};
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache, SwashContent, Weight};

use crate::canvas::Canvas;
use crate::fonts::{FontLibrary, Trak};
use crate::theme::FontRole;

pub(crate) use size::TextSize;
pub(crate) use style::TextStyle;

pub(crate) struct TextPainter {
    /// 字体库与回退链。
    font_system: FontSystem,

    candidate_family: Option<String>,

    annotation_family: Option<String>,

    candidate_weights: [Weight; 2],

    annotation_weights: [Weight; 2],

    /// 字形位图缓存（按字体、字号、亚像素位移）。
    cache: SwashCache,

    /// 复用的单行缓冲。
    buffer: Buffer,

    /// 每张字体的字距表（没有的记 `None`），按字形所用字体各查各的，与 CoreText 一致。
    tracking: HashMap<ID, Option<Trak>>,

    /// 覆盖率 gamma 查找表，按 gamma 值缓存。
    gamma_tables: HashMap<u32, Box<[u8; 256]>>,

    /// 没有粗体面或可变字重轴的字体，需要合成粗体。
    synthetic_bold: HashMap<ID, bool>,
}

impl TextPainter {
    pub(crate) fn new(library: FontLibrary) -> Self {
        let candidate_family = library.candidate_family.clone();
        let annotation_family = library.annotation_family.clone();
        let mut font_system = library.into_font_system();
        let candidate_weights = family_weights(&mut font_system, candidate_family.as_deref());
        let annotation_weights = family_weights(&mut font_system, annotation_family.as_deref());
        let buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 19.0));
        Self {
            font_system,
            candidate_family,
            annotation_family,
            candidate_weights,
            annotation_weights,
            cache: SwashCache::new(),
            buffer,
            tracking: HashMap::new(),
            gamma_tables: HashMap::new(),
            synthetic_bold: HashMap::new(),
        }
    }

    /// 光学字号（点）：SF 这类带 `opsz` 轴的字体在小字号用文本视觉尺寸，CoreText 对系统字体自动做，这里要显式给。
    /// 现在是整个画笔一个值（cosmic-text 的字体实例缓存没按它分键），候选窗几种字号都在 20 pt 以下，落到同一档。
    pub(crate) fn set_optical_size(&mut self, points: Option<f32>) {
        self.font_system.set_optical_size(points);
    }

    /// 一段文字的宽高（像素）。高度就是行高。
    pub(crate) fn measure(&mut self, text: &str, style: &TextStyle) -> TextSize {
        self.shape(text, style);
        let mut width = 0.0_f32;
        for run in self.buffer.layout_runs() {
            let tracked: f32 = run
                .glyphs
                .iter()
                .map(|glyph| {
                    tracking_px(&self.font_system, &mut self.tracking, glyph.font_id, style)
                })
                .sum();
            width = width.max(run.line_w + tracked);
        }
        TextSize {
            width: width
                + if text.is_empty() {
                    0.0
                } else {
                    bold_inset(style)
                },
            height: style.line_height,
        }
    }

    /// 把文字画到 `(x, y)`，`y` 是行框顶边。返回宽度。
    pub(crate) fn draw(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        style: &TextStyle,
        x: f32,
        y: f32,
    ) -> f32 {
        self.shape(text, style);
        let mut width = 0.0_f32;
        let mut strike: Option<(f32, f32)> = None;
        let mut underline: Option<(f32, f32)> = None;
        for run in self.buffer.layout_runs() {
            let baseline = y + run.line_y;
            // 每个字形画完把它那份字距累加到后面所有字形的 x 上
            let mut tracked = 0.0_f32;
            for glyph in run.glyphs {
                let physical = glyph.physical((x + tracked, y), 1.0);
                tracked += tracking_px(&self.font_system, &mut self.tracking, glyph.font_id, style);
                let synthetic = style.bold
                    && *self.synthetic_bold.entry(glyph.font_id).or_insert_with(|| {
                        let regular = self
                            .font_system
                            .db()
                            .face(glyph.font_id)
                            .is_some_and(|face| face.weight < Weight::SEMIBOLD);
                        regular
                            && self
                                .font_system
                                .get_font(glyph.font_id, glyph.font_weight)
                                .is_some_and(|font| {
                                    !font
                                        .as_swash()
                                        .variations()
                                        .any(|axis| axis.tag() == u32::from_be_bytes(*b"wght"))
                                })
                    });
                let Some(image) = self
                    .cache
                    .get_image(&mut self.font_system, physical.cache_key)
                else {
                    continue;
                };
                let gx = physical.x + image.placement.left;
                let gy = run.line_y.round() as i32 + physical.y - image.placement.top;
                let (w, h) = (image.placement.width, image.placement.height);
                match image.content {
                    SwashContent::Mask => {
                        let table = gamma_table(&mut self.gamma_tables, style.gamma);
                        let data: Vec<u8> = image.data.iter().map(|&c| table[c as usize]).collect();
                        if synthetic {
                            let inset = bold_inset(style) as u32;
                            let expanded = embolden_mask(&data, w, h, inset);
                            canvas.blend_mask(gx, gy, w + inset, h, &expanded, style.color);
                        } else {
                            canvas.blend_mask(gx, gy, w, h, &data, style.color);
                        }
                    }
                    SwashContent::Color => canvas.blend_rgba(gx, gy, w, h, &image.data),
                    // 没有申请亚像素格式，不会出现
                    SwashContent::SubpixelMask => {}
                }
            }
            width = width.max(run.line_w + tracked);
            if style.strike {
                strike = Some((baseline, run.line_w + tracked));
            }
            if style.underline {
                underline = Some((baseline, run.line_w + tracked));
            }
        }
        if let Some((baseline, line_w)) = strike {
            // 删除线穿过小写字母中部
            let thickness = (style.size / 14.0).max(1.0);
            let line_y = baseline - style.size * 0.3;
            canvas.fill_rect(x, line_y, line_w, thickness, style.color);
        }
        if let Some((baseline, line_w)) = underline {
            // 下划线压在基线下面一点，与删除线同一套粗细
            let thickness = (style.size / 14.0).max(1.0);
            let line_y = baseline + style.size * 0.14;
            canvas.fill_rect(x, line_y, line_w, thickness, style.color);
        }
        width
            + if text.is_empty() {
                0.0
            } else {
                bold_inset(style)
            }
    }

    /// 每个字形用的字族名（相邻相同的合并），拿来核对中日字形与 emoji 回退到了哪家字体。
    pub(crate) fn trace_families(&mut self, text: &str, style: &TextStyle) -> Vec<String> {
        self.shape(text, style);
        let mut names: Vec<String> = Vec::new();
        for run in self.buffer.layout_runs() {
            for glyph in run.glyphs {
                let name = self
                    .font_system
                    .db()
                    .face(glyph.font_id)
                    .and_then(|face| face.families.first().map(|(n, _)| n.clone()))
                    .unwrap_or_else(|| "?".to_owned());
                if names.last() != Some(&name) {
                    names.push(name);
                }
            }
        }
        names
    }

    fn shape(&mut self, text: &str, style: &TextStyle) {
        let weight = match style.role {
            FontRole::Ui => {
                if style.bold {
                    Weight::BOLD
                } else {
                    Weight::NORMAL
                }
            }
            FontRole::Candidate => self.candidate_weights[usize::from(style.bold)],
            FontRole::Annotation => self.annotation_weights[usize::from(style.bold)],
        };
        let family = match style.role {
            FontRole::Ui => None,
            FontRole::Candidate => self.candidate_family.as_deref(),
            FontRole::Annotation => self.annotation_family.as_deref(),
        };
        let attrs = Attrs::new()
            .family(family.map_or(Family::SansSerif, Family::Name))
            .weight(weight)
            .color(style.color.to_cosmic());
        self.buffer
            .set_metrics(Metrics::new(style.size, style.line_height));
        self.buffer.set_size(None, None);
        self.buffer.set_text(text, &attrs, Shaping::Advanced, None);
        self.buffer.shape_until_scroll(&mut self.font_system, false);
    }
}

/// cosmic-text 的首选字族要求字重精确匹配；静态字体用最接近的现有面，再由栅格器合成粗体，避免换成别的字族。
fn family_weights(font_system: &mut FontSystem, family: Option<&str>) -> [Weight; 2] {
    [Weight::NORMAL, Weight::BOLD].map(|requested| {
        let Some(family) = family else {
            return requested;
        };
        let Some((id, actual)) = font_system
            .db()
            .query(&Query {
                families: &[Family::Name(family)],
                weight: requested,
                ..Query::default()
            })
            .and_then(|id| font_system.db().face(id).map(|face| (id, face.weight)))
        else {
            return requested;
        };
        let variable = font_system.get_font(id, requested).is_some_and(|font| {
            font.as_swash()
                .variations()
                .any(|axis| axis.tag() == u32::from_be_bytes(*b"wght"))
        });
        if variable { requested } else { actual }
    })
}

/// 给合成粗体留出笔画宽度，测量与绘制使用同一数值。
fn bold_inset(style: &TextStyle) -> f32 {
    if style.bold {
        (style.size / 32.0).ceil()
    } else {
        0.0
    }
}

/// 水平扩张覆盖率遮罩，保留灰阶边缘；彩色 emoji 不经过这里。
fn embolden_mask(data: &[u8], width: u32, height: u32, inset: u32) -> Vec<u8> {
    let stride = width + inset;
    let mut out = vec![0; (stride * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let coverage = data[(y * width + x) as usize];
            for dx in 0..=inset {
                let pixel = &mut out[(y * stride + x + dx) as usize];
                *pixel = (*pixel).max(coverage);
            }
        }
    }
    out
}

/// 某张字体在这个字号下每个字形要加的间距（像素）；第一次用到时解析它的 `trak` 表。
fn tracking_px(
    font_system: &FontSystem,
    cache: &mut HashMap<ID, Option<Trak>>,
    font_id: ID,
    style: &TextStyle,
) -> f32 {
    let trak = cache.entry(font_id).or_insert_with(|| {
        font_system
            .db()
            .with_face_data(font_id, Trak::parse)
            .flatten()
    });
    trak.as_ref()
        .map_or(0.0, |trak| trak.tracking_em(style.points) * style.size)
}

/// 覆盖率 → gamma 校正后的覆盖率。
fn gamma_table(cache: &mut HashMap<u32, Box<[u8; 256]>>, gamma: f32) -> &[u8; 256] {
    cache.entry(gamma.to_bits()).or_insert_with(|| {
        let mut table = [0u8; 256];
        for (i, out) in table.iter_mut().enumerate() {
            *out = ((i as f32 / 255.0).powf(gamma) * 255.0).round() as u8;
        }
        Box::new(table)
    })
}
