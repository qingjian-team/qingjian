//! 文字测绘：cosmic-text 整形 + swash 栅格，单行、像素坐标；普通字形走覆盖率遮罩，彩色 emoji 走 RGBA 位图。

mod decoration;
pub mod font;
mod measure_key;
mod size;
mod style;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cosmic_text::fontdb::{Family, ID};
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Style, SwashCache, SwashContent};

use crate::canvas::Canvas;
use crate::fonts::{FontLibrary, Trak, UI_FAMILY, WeightAliases, drawable};

pub use decoration::Decoration;
pub use size::TextSize;
pub use style::TextStyle;

use font::{FamilyList, FamilyTable};
use measure_key::MeasureKey;

/// 量宽缓存的上限，满了整个清掉（设置页一屏的文字在几百条以内）。
const MEASURED_LIMIT: usize = 4096;

/// 截短文字时补在末尾的省略号。
const ELLIPSIS: &str = "…";

pub struct TextPainter {
    /// 字体库与回退链。
    font_system: FontSystem,

    /// 字形位图缓存（按字体、字号、亚像素位移）。
    cache: SwashCache,

    /// 复用的单行缓冲。
    buffer: Buffer,

    /// 每张字体的字距表（没有的记 `None`），按字形所用字体各查各的，与 CoreText 一致。
    tracking: HashMap<ID, Option<Trak>>,

    /// 覆盖率 gamma 查找表，按 gamma 值缓存。
    gamma_tables: HashMap<u32, Box<[u8; 256]>>,

    /// 当前的字族表；换了表或加载了新字体就重新挑。
    families: Option<Arc<FamilyTable>>,

    /// 字族表每条回退链挑中的字族名（字体库里的写法），`None` 用界面字体。
    resolved: Vec<Option<String>>,

    /// 要求加载过的字体文件（加载失败的也记，不反复试）。
    loaded: HashSet<PathBuf>,

    /// 量过的文字：每帧重建整棵树时同样的文字不再整形。字体、字族、光学字号变了就清。
    measured: HashMap<MeasureKey, TextSize>,

    /// 给字族补的缺档字重，加载了新字体就重补。
    weight_aliases: WeightAliases,
}

impl TextPainter {
    pub fn new(library: FontLibrary) -> Self {
        let mut font_system = library.into_font_system();
        let mut weight_aliases = WeightAliases::default();
        weight_aliases.refresh(font_system.db_mut());
        let buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 19.0));
        Self {
            font_system,
            cache: SwashCache::new(),
            buffer,
            tracking: HashMap::new(),
            gamma_tables: HashMap::new(),
            families: None,
            resolved: Vec::new(),
            loaded: HashSet::new(),
            measured: HashMap::new(),
            weight_aliases,
        }
    }

    /// 字体库里有没有这个字族（不分大小写）。
    pub fn has_family(&self, name: &str) -> bool {
        self.family_name(name).is_some()
    }

    /// 字体库里这个字族的写法。
    fn family_name(&self, name: &str) -> Option<String> {
        self.font_system.db().faces().find_map(|face| {
            face.families
                .iter()
                .find(|(family, _)| family.eq_ignore_ascii_case(name))
                .map(|(family, _)| family.clone())
        })
    }

    /// 加载字体文件；每个文件只试一次。加载了新文件就重新挑字族。
    pub fn load_fonts(&mut self, paths: &[PathBuf]) {
        let mut added = false;
        for path in paths {
            if !self.loaded.insert(path.clone()) {
                continue;
            }
            added |= load_font(&mut self.font_system, path);
        }
        if added {
            self.weight_aliases.refresh(self.font_system.db_mut());
            self.families = None;
            self.measured.clear();
        }
    }

    /// 换成这张字族表：每条回退链挑第一个字体库里有的字族，同一张表不重挑。
    pub fn use_families(&mut self, families: &Arc<FamilyTable>) {
        if self
            .families
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, families))
        {
            return;
        }
        self.resolved = families
            .chains()
            .iter()
            .map(|chain| {
                for name in &chain.0 {
                    if FamilyList::is_system(name) {
                        return None;
                    }
                    if let Some(found) = self.family_name(name) {
                        return Some(found);
                    }
                }
                tracing::debug!(chain = ?chain.0, "回退链里的字体都没装，用界面字体");
                None
            })
            .collect();
        self.families = Some(Arc::clone(families));
        self.measured.clear();
    }

    /// 光学字号（点）：SF 这类带 `opsz` 轴的字体在小字号用文本视觉尺寸，CoreText 对系统字体自动做，这里要显式给。
    /// 现在是整个画笔一个值（cosmic-text 的字体实例缓存没按它分键），界面里的小字号都在 20 pt 以下，落到同一档。
    pub fn set_optical_size(&mut self, points: Option<f32>) {
        self.font_system.set_optical_size(points);
        self.measured.clear();
    }

    /// 一段文字的宽高（像素），一行到底。高度就是行高。
    pub fn measure(&mut self, text: &str, style: &TextStyle) -> TextSize {
        self.measure_within(text, style, None)
    }

    /// 按 `width` 折行后的宽高（像素）：宽是最长那行，高是行数乘行高。`None` 不折行。
    pub fn measure_within(
        &mut self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
    ) -> TextSize {
        let key = MeasureKey::new(text, style, width);
        if let Some(size) = self.measured.get(&key) {
            return *size;
        }
        let size = self.measure_uncached(text, style, width);
        if self.measured.len() >= MEASURED_LIMIT {
            self.measured.clear();
        }
        self.measured.insert(key, size);
        size
    }

    fn measure_uncached(&mut self, text: &str, style: &TextStyle, wrap: Option<f32>) -> TextSize {
        self.shape(text, style, wrap);
        let mut width = 0.0_f32;
        let mut baseline = None;
        let mut lines = 0_u16;
        for run in self.buffer.layout_runs() {
            lines += 1;
            baseline.get_or_insert(run.line_y.round());
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
            width,
            // 不折行的文字按一行算高（换行符也不撑高，与以前一致）
            height: style.line_height * f32::from(if style.wrap { lines.max(1) } else { 1 }),
            baseline: baseline.unwrap_or(0.0),
        }
    }

    /// 一行放进 `width` 像素：放得下原样返回，放不下截到能放下的最长前缀再补「…」（一个字都放不下就只剩「…」）。
    pub fn fit<'a>(&mut self, text: &'a str, style: &TextStyle, width: f32) -> Cow<'a, str> {
        // 量出来的宽与布局给的宽可能差一点舍入，放宽半个像素
        let width = width + 0.5;
        if self.measure(text, style).width <= width {
            return Cow::Borrowed(text);
        }
        let ends: Vec<usize> = text.char_indices().map(|(i, _)| i).skip(1).collect();
        // 二分找最长的放得下的前缀（`ends[i]` 是前 i + 1 个字的结尾）
        let (mut low, mut high) = (0, ends.len());
        while low < high {
            let middle = (low + high).div_ceil(2);
            let candidate = format!("{}{ELLIPSIS}", &text[..ends[middle - 1]]);
            if self.measure(&candidate, style).width <= width {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        let prefix = if low == 0 { "" } else { &text[..ends[low - 1]] };
        Cow::Owned(format!("{}{ELLIPSIS}", prefix.trim_end()))
    }

    /// 把文字画到 `(x, y)`，`y` 是行框顶边，一行到底。返回宽度。
    pub fn draw(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        style: &TextStyle,
        x: f32,
        y: f32,
    ) -> f32 {
        self.draw_within(canvas, text, style, (x, y), None)
    }

    /// 同 [`Self::draw`]，按 `width` 折行（`None` 不折）。
    pub fn draw_within(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        style: &TextStyle,
        (x, y): (f32, f32),
        width: Option<f32>,
    ) -> f32 {
        self.shape(text, style, width);
        if let Some((stroke_width, stroke_color)) = style.stroke {
            self.draw_stroke(canvas, style, x, y, stroke_width, stroke_color);
        }
        let mut width = 0.0_f32;
        let mut last_line: Option<(f32, f32)> = None;
        for run in self.buffer.layout_runs() {
            // 每个字形画完把它那份字距累加到后面所有字形的 x 上
            let mut tracked = 0.0_f32;
            for glyph in run.glyphs {
                let physical = glyph.physical((x + tracked, y), 1.0);
                tracked += tracking_px(&self.font_system, &mut self.tracking, glyph.font_id, style);
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
                        canvas.blend_mask(gx, gy, w, h, &data, style.color);
                    }
                    SwashContent::Color => canvas.blend_rgba(gx, gy, w, h, &image.data),
                    // 没有申请亚像素格式，不会出现
                    SwashContent::SubpixelMask => {}
                }
            }
            width = width.max(run.line_w + tracked);
            last_line = Some((y + run.line_y, run.line_w + tracked));
        }
        if let (Some(decoration), Some((baseline, line_width))) = (style.decoration, last_line) {
            canvas.fill_rect(
                x,
                baseline + style.size * decoration.offset(),
                line_width,
                Decoration::thickness(style.size),
                style.color,
            );
        }
        width
    }

    /// 描边：整段文字的字形轮廓拼成一条路径，按两倍宽描一次（字形随后压在上面，露出来的就是向外的宽度）。
    /// 一条路径描一次，半透明的描边在字形相交处不会叠深。没有轮廓的字形（位图 emoji）不描。
    fn draw_stroke(
        &mut self,
        canvas: &mut Canvas,
        style: &TextStyle,
        x: f32,
        y: f32,
        width: f32,
        color: crate::color::Color,
    ) {
        let mut path = tiny_skia::PathBuilder::new();
        for run in self.buffer.layout_runs() {
            let baseline = y + run.line_y.round();
            let mut tracked = 0.0_f32;
            for glyph in run.glyphs {
                let physical = glyph.physical((x + tracked, y), 1.0);
                let origin_x = x + tracked + glyph.x + glyph.font_size * glyph.x_offset;
                let origin_y = baseline + glyph.y - glyph.font_size * glyph.y_offset;
                tracked += tracking_px(&self.font_system, &mut self.tracking, glyph.font_id, style);
                let Some(commands) = self
                    .cache
                    .get_outline_commands(&mut self.font_system, physical.cache_key)
                else {
                    continue;
                };
                for command in commands {
                    let at = |p: zeno::Point| (origin_x + p.x, origin_y - p.y);
                    match *command {
                        zeno::Command::MoveTo(p) => {
                            let (px, py) = at(p);
                            path.move_to(px, py);
                        }
                        zeno::Command::LineTo(p) => {
                            let (px, py) = at(p);
                            path.line_to(px, py);
                        }
                        zeno::Command::QuadTo(c, p) => {
                            let ((cx, cy), (px, py)) = (at(c), at(p));
                            path.quad_to(cx, cy, px, py);
                        }
                        zeno::Command::CurveTo(c1, c2, p) => {
                            let ((ax, ay), (bx, by), (px, py)) = (at(c1), at(c2), at(p));
                            path.cubic_to(ax, ay, bx, by, px, py);
                        }
                        zeno::Command::Close => path.close(),
                    }
                }
            }
        }
        if let Some(path) = path.finish() {
            canvas.stroke_outline(&path, width * 2.0, color);
        }
    }

    /// 每个字形用的字族名（相邻相同的合并），拿来核对中日字形与 emoji 回退到了哪家字体。
    pub fn trace_families(&mut self, text: &str, style: &TextStyle) -> Vec<String> {
        self.shape(text, style, None);
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

    fn shape(&mut self, text: &str, style: &TextStyle, width: Option<f32>) {
        let family = match self.resolved.get(usize::from(style.family.0)) {
            Some(Some(name)) => Family::Name(name),
            _ => UI_FAMILY,
        };
        let attrs = Attrs::new()
            .family(family)
            .weight(cosmic_text::Weight(style.weight.0))
            .style(if style.italic {
                Style::Italic
            } else {
                Style::Normal
            })
            .color(style.color.to_cosmic());
        self.buffer
            .set_metrics(Metrics::new(style.size, style.line_height));
        self.buffer.set_size(width, None);
        self.buffer.set_text(text, &attrs, Shaping::Advanced, None);
        self.buffer.shape_until_scroll(&mut self.font_system, false);
    }
}

/// 文件能解析、有 swash 能画的轮廓（见 [`drawable`]）就加载进字体库；返回是否加载了。
fn load_font(font_system: &mut FontSystem, path: &Path) -> bool {
    if !drawable(path) {
        tracing::warn!(path = %path.display(), "字体文件不存在、读不进来或没有能画的轮廓，跳过");
        return false;
    }
    match font_system.db_mut().load_font_file(path) {
        Ok(()) => {
            tracing::debug!(path = %path.display(), "加载字体文件");
            true
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "字体文件读不进来");
            false
        }
    }
}

/// 某张字体在这个字号下每个字形要加的间距（像素）；第一次用到时解析它的 `trak` 表。
/// 折行的段落不加：cosmic-text 折行时不知道这份间距，加了会超出折行宽度。
fn tracking_px(
    font_system: &FontSystem,
    cache: &mut HashMap<ID, Option<Trak>>,
    font_id: ID,
    style: &TextStyle,
) -> f32 {
    if style.wrap {
        return 0.0;
    }
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
