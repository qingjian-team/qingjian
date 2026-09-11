//! 候选窗口主题：字体、颜色、间距。视觉层级对齐 macOS 端（候选词最深、译文次之、词性最浅、序号弱化、
//! 生词橙、云联想 teal、高亮蓝）；平台底层用 Windows 的：系统 UI 字体、贴近 mac 语义色的固定值（GDI 没有
//! 随外观切换的语义色，浅 / 深各写一套）、度量按 DPI 缩放。

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DeleteObject,
    FF_DONTCARE, HFONT, OUT_TT_PRECIS, VARIABLE_PITCH,
};
use windows::core::w;

/// 常规字重；windows crate 未导出。
const FW_NORMAL: i32 = 400;

/// COLORREF 低位到高位是 R、G、B。
const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

/// 一套配色 + 按 DPI 造好的字体。字体是 GDI 资源，`Drop` 里删。
pub(crate) struct Theme {
    pub text_font: HFONT,

    pub annotation_font: HFONT,

    pub index_font: HFONT,

    pub text_color: COLORREF,

    pub gloss_color: COLORREF,

    pub pos_color: COLORREF,

    /// 生词译文，比普通译文醒目。
    pub fresh_color: COLORREF,

    pub index_color: COLORREF,

    /// 云联想的云朵与文字。
    pub cloud_color: COLORREF,

    pub background: COLORREF,

    /// 当前候选的高亮底色（mac 的半透明蓝预混成不透明值，GDI 无 alpha）。
    pub highlight: COLORREF,

    /// 窗口内边距（已按 DPI 缩放）。
    pub padding: i32,

    /// 行内上下留白。
    pub row_padding: i32,

    /// 列间距。
    pub column_gap: i32,

    /// 窗口圆角半径。
    pub corner_radius: i32,
}

impl Theme {
    /// `dpi` 取 `GetDpiForWindow`（96 为 100%）。
    pub(crate) fn new(dpi: u32, dark: bool) -> Self {
        let scale = |px: i32| (px * dpi as i32) / 96;
        // 负高度表示字符高度（不含内部行距）。
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

impl Drop for Theme {
    fn drop(&mut self) {
        for font in [self.text_font, self.annotation_font, self.index_font] {
            if !font.is_invalid() {
                // SAFETY: 由本主题的 CreateFontW 造出、未被别处持有。
                let _ = unsafe { DeleteObject(font.into()) };
            }
        }
    }
}

/// 浅色 / 深色各一套。
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
    /// 贴近 mac light：label / secondary / tertiary label、systemOrange、systemTeal、窗口背景。
    fn light() -> Self {
        Self {
            text_color: rgb(0x1d, 0x1d, 0x1f),
            gloss_color: rgb(0x6b, 0x6b, 0x70),
            pos_color: rgb(0xa0, 0xa0, 0xa6),
            fresh_color: rgb(0xff, 0x95, 0x00),
            index_color: rgb(0xa0, 0xa0, 0xa6),
            cloud_color: rgb(0x30, 0xb0, 0xc7),
            background: rgb(0xf8, 0xf8, 0xf8),
            // sRGB(0,0.48,1.0) @16% 叠在浅背景上。
            highlight: rgb(0xcf, 0xe4, 0xf9),
        }
    }

    /// 贴近 mac dark。
    fn dark() -> Self {
        Self {
            text_color: rgb(0xf5, 0xf5, 0xf7),
            gloss_color: rgb(0xae, 0xae, 0xb2),
            pos_color: rgb(0x8e, 0x8e, 0x93),
            fresh_color: rgb(0xff, 0x9f, 0x0a),
            index_color: rgb(0x8e, 0x8e, 0x93),
            cloud_color: rgb(0x40, 0xc8, 0xe0),
            background: rgb(0x2a, 0x2a, 0x2c),
            // 深背景上按约 28% 预混才够醒目。
            highlight: rgb(0x2f, 0x4d, 0x72),
        }
    }
}

/// 微软雅黑 UI，缺字由 GDI 字体链回落。`height` 为负的字符高度。
fn create_ui_font(height: i32) -> HFONT {
    // SAFETY: 参数都是合法常量；facename 是静态宽字符串。
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
