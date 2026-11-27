//! 进程内候选窗的画法主题：系统字族的字体、一套内置配色、间距。
//!
//! TSF 进程里读不到用户主题目录（AppContainer 宿主尤其读不到），也不加载主题字体文件，
//! 所以这里只用系统字族（`Microsoft YaHei UI`）与内置的浅 / 深配色。外观与 Server 候选窗
//! 不必逐像素一致，选词、翻页、高亮这些行为一致即可。

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DeleteObject,
    FF_DONTCARE, HFONT, OUT_TT_PRECIS, VARIABLE_PITCH,
};
use windows::core::{PCWSTR, w};

/// 常规字重；windows crate 未导出。
const FW_NORMAL: i32 = 400;

/// COLORREF 低位到高位是 R、G、B。
pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

/// 浅色 / 深色两套配色。取色沿用 Server 候选窗的 GDI 主题，深浅各写一套（GDI 没有语义色）。
struct Palette {
    text_color: COLORREF,
    gloss_color: COLORREF,
    pos_color: COLORREF,
    fresh_color: COLORREF,
    index_color: COLORREF,
    cloud_color: COLORREF,
    background: COLORREF,
    border: COLORREF,
    highlight: COLORREF,
}

impl Palette {
    /// 贴近 Server 浅色主题。
    fn light() -> Self {
        Self {
            text_color: rgb(0x1d, 0x1d, 0x1f),
            gloss_color: rgb(0x6b, 0x6b, 0x70),
            pos_color: rgb(0xa0, 0xa0, 0xa6),
            fresh_color: rgb(0xff, 0x95, 0x00),
            index_color: rgb(0xa0, 0xa0, 0xa6),
            cloud_color: rgb(0x30, 0xb0, 0xc7),
            background: rgb(0xf8, 0xf8, 0xf8),
            border: rgb(0xd8, 0xd8, 0xdd),
            highlight: rgb(0xcf, 0xe4, 0xf9),
        }
    }

    /// 贴近 Server 深色主题。
    fn dark() -> Self {
        Self {
            text_color: rgb(0xf5, 0xf5, 0xf7),
            gloss_color: rgb(0xae, 0xae, 0xb2),
            pos_color: rgb(0x8e, 0x8e, 0x93),
            fresh_color: rgb(0xff, 0x9f, 0x0a),
            index_color: rgb(0x8e, 0x8e, 0x93),
            cloud_color: rgb(0x40, 0xc8, 0xe0),
            background: rgb(0x2a, 0x2a, 0x2c),
            border: rgb(0x3c, 0x3c, 0x40),
            highlight: rgb(0x2f, 0x4d, 0x72),
        }
    }
}

/// 一套配色 + 按 DPI 造好的字体。字体是 GDI 资源，`Drop` 里删。
pub(crate) struct Theme {
    pub(crate) text_font: HFONT,

    pub(crate) annotation_font: HFONT,

    pub(crate) index_font: HFONT,

    pub(crate) text_color: COLORREF,

    pub(crate) gloss_color: COLORREF,

    pub(crate) pos_color: COLORREF,

    /// 生词译文，比普通译文醒目。
    pub(crate) fresh_color: COLORREF,

    pub(crate) index_color: COLORREF,

    /// 云联想的云朵与文字。
    pub(crate) cloud_color: COLORREF,

    pub(crate) background: COLORREF,

    /// 窗口描边（GDI 画不出柔和投影，用一圈淡边把窗口与背景分开）。
    pub(crate) border: COLORREF,

    /// 当前候选的高亮底色。
    pub(crate) highlight: COLORREF,

    /// 窗口内边距（已按 DPI 缩放）。
    pub(crate) padding: i32,

    /// 行内上下留白。
    pub(crate) row_padding: i32,

    /// 列间距。
    pub(crate) column_gap: i32,

    /// 窗口圆角半径。
    pub(crate) corner_radius: i32,
}

impl Theme {
    /// `candidate` / `annotation` 是按 `dpi` 换算好的像素高度（负值在 [`create_font`] 里加）：
    /// 字号来自 Server 下发的设置，缺省沿用 Server 的 16 / 12 像素。
    pub(crate) fn new(
        dpi: u32,
        dark: bool,
        candidate: i32,
        annotation: i32,
        weight: Option<u16>,
    ) -> Self {
        let scale = |px: i32| (px * dpi as i32) / 96;
        let weight = weight.map_or(FW_NORMAL, i32::from);
        let font = |px: i32| create_font(-px.max(1), weight);
        let palette = if dark {
            Palette::dark()
        } else {
            Palette::light()
        };
        Self {
            text_font: font(candidate),
            annotation_font: font(annotation),
            index_font: font((annotation - 1).max(1)),
            text_color: palette.text_color,
            gloss_color: palette.gloss_color,
            pos_color: palette.pos_color,
            fresh_color: palette.fresh_color,
            index_color: palette.index_color,
            cloud_color: palette.cloud_color,
            background: palette.background,
            border: palette.border,
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
                let _ = unsafe { DeleteObject(font.into()) };
            }
        }
    }
}

/// 缺字由 GDI 字体链回落。`height` 为负的字符高度。
fn create_font(height: i32, weight: i32) -> HFONT {
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_TT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            (VARIABLE_PITCH.0 | FF_DONTCARE.0) as u32,
            PCWSTR(w!("Microsoft YaHei UI").0),
        )
    }
}
