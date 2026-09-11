//! 候选窗口主题：字体、颜色、间距。集中所有可视参数，对齐 macOS 端的视觉层级——
//! 候选词最深、译文稍浅、词性最浅、序号弱化、生词橙、云联想 teal、当前候选蓝色高亮。
//!
//! 与 mac 的差别只在「平台底层」：字体用 Windows 的系统 UI 字体（微软雅黑 UI，缺字回落 Segoe UI），
//! 颜色用贴近 mac 语义色观感的固定值（GDI 没有 NSColor 那套随外观自动切换的语义色，浅 / 深各写一套，跟随系统或配置切换）。
//! 度量沿用 mac 的像素数，按窗口 DPI 缩放，HiDPI 上不发虚。

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DeleteObject,
    FF_DONTCARE, HFONT, OUT_TT_PRECIS, VARIABLE_PITCH,
};
use windows::core::w;

/// 把 8 位 RGB 分量拼成 GDI 的 `COLORREF`（低位到高位是 R、G、B）。
const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

/// 候选窗口主题（按深浅选一套配色 + 按 DPI 造好的字体）。字体是 GDI 资源，`Drop` 里删。
pub(crate) struct Theme {
    /// 候选词字体。
    pub text_font: HFONT,

    /// 译文与词性字体。
    pub annotation_font: HFONT,

    /// 序号字体。
    pub index_font: HFONT,

    /// 候选词颜色（最深）。
    pub text_color: COLORREF,

    /// 译文颜色（次浅）。
    pub gloss_color: COLORREF,

    /// 词性颜色（最浅）。
    pub pos_color: COLORREF,

    /// 生词译文颜色：比普通译文醒目，提醒「这个词还没见过几次」。
    pub fresh_color: COLORREF,

    /// 序号颜色（弱化）。
    pub index_color: COLORREF,

    /// 云联想的云朵与文字颜色。
    pub cloud_color: COLORREF,

    /// 窗口背景。
    pub background: COLORREF,

    /// 当前候选的高亮底色（已把 mac 的半透明蓝预混成不透明值，GDI 无 alpha）。
    pub highlight: COLORREF,

    /// 窗口内边距（已按 DPI 缩放的像素）。
    pub padding: i32,

    /// 行内上下留白。
    pub row_padding: i32,

    /// 序号与候选词、候选词与译文之间的列间距。
    pub column_gap: i32,

    /// 窗口圆角半径。
    pub corner_radius: i32,
}

impl Theme {
    /// 按窗口 DPI 与深浅造一套主题。`dpi` 取 `GetDpiForWindow`（96 为 100%）；`dark` 为真用深色调色板。
    pub(crate) fn new(dpi: u32, dark: bool) -> Self {
        // mac 用的像素尺寸，这里按 DPI 缩放：px * dpi / 96。
        let scale = |px: i32| (px * dpi as i32) / 96;
        // 字体高度用负值表示「字符高度」（区别于含内部行距的单元格高度）。
        let font = |px: i32| create_ui_font(-scale(px));
        let palette = if dark {
            Palette::dark()
        } else {
            Palette::light()
        };
        Self {
            text_font: font(16),
            annotation_font: font(12),
            index_font: font(11),
            text_color: palette.text_color,
            gloss_color: palette.gloss_color,
            pos_color: palette.pos_color,
            fresh_color: palette.fresh_color,
            index_color: palette.index_color,
            cloud_color: palette.cloud_color,
            background: palette.background,
            highlight: palette.highlight,
            padding: scale(8),
            row_padding: scale(4),
            column_gap: scale(8),
            corner_radius: scale(8),
        }
    }
}

/// 一套配色（浅色 / 深色各一）。层级角色对齐 macOS，取贴近其 light / dark 语义色观感的固定值——GDI 没有
/// NSColor 那套随外观自动切换的语义色，只能各写一套。高亮已把 mac 的半透明蓝按各自背景预混成不透明值。
struct Palette {
    text_color: COLORREF,
    gloss_color: COLORREF,
    pos_color: COLORREF,
    fresh_color: COLORREF,
    index_color: COLORREF,
    cloud_color: COLORREF,
    background: COLORREF,
    highlight: COLORREF,
}

impl Palette {
    /// 浅色：贴近 mac light（label / secondary / tertiary label、systemOrange、systemTeal、窗口背景）。
    fn light() -> Self {
        Self {
            text_color: rgb(0x1d, 0x1d, 0x1f),
            gloss_color: rgb(0x6b, 0x6b, 0x70),
            pos_color: rgb(0xa0, 0xa0, 0xa6),
            fresh_color: rgb(0xff, 0x95, 0x00),
            index_color: rgb(0xa0, 0xa0, 0xa6),
            cloud_color: rgb(0x30, 0xb0, 0xc7),
            background: rgb(0xf8, 0xf8, 0xf8),
            // mac 的高亮是 sRGB(0,0.48,1.0) @16% 叠在浅背景上；GDI 无 alpha，预混成不透明值。
            highlight: rgb(0xcf, 0xe4, 0xf9),
        }
    }

    /// 深色：贴近 mac dark（近白 label、次浅 / 更浅灰、systemOrange/Teal 的深色变体、深灰面板背景）。
    fn dark() -> Self {
        Self {
            text_color: rgb(0xf5, 0xf5, 0xf7),
            gloss_color: rgb(0xae, 0xae, 0xb2),
            pos_color: rgb(0x8e, 0x8e, 0x93),
            fresh_color: rgb(0xff, 0x9f, 0x0a),
            index_color: rgb(0x8e, 0x8e, 0x93),
            cloud_color: rgb(0x40, 0xc8, 0xe0),
            background: rgb(0x2a, 0x2a, 0x2c),
            // 蓝高亮按更高不透明度（约 28%）预混在深背景上，深色下才够醒目。
            highlight: rgb(0x2f, 0x4d, 0x72),
        }
    }
}

impl Drop for Theme {
    fn drop(&mut self) {
        for font in [self.text_font, self.annotation_font, self.index_font] {
            if !font.is_invalid() {
                // SAFETY: 这些 HFONT 由本主题的 CreateFontW 造出、未被别处持有。
                let _ = unsafe { DeleteObject(font.into()) };
            }
        }
    }
}

/// 造一支系统 UI 字体：微软雅黑 UI（缺字由 GDI 字体链回落）。`height` 为负的字符高度。
fn create_ui_font(height: i32) -> HFONT {
    // SAFETY: 参数都是合法常量；facename 是静态宽字符串字面量。
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            FW_NORMAL,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_TT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            (VARIABLE_PITCH.0 | FF_DONTCARE.0) as u32,
            w!("Microsoft YaHei UI"),
        )
    }
}

/// `FW_NORMAL`（常规字重 400）。windows crate 未导出该常量，直接给值。
const FW_NORMAL: i32 = 400;
