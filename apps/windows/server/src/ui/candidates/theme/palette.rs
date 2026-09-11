//! 浅色 / 深色配色。

use windows::Win32::Foundation::COLORREF;

use super::rgb;

/// 浅色 / 深色各一套。
pub(super) struct Palette {
    pub(super) text_color: COLORREF,
    pub(super) gloss_color: COLORREF,
    pub(super) pos_color: COLORREF,
    pub(super) fresh_color: COLORREF,
    pub(super) index_color: COLORREF,
    pub(super) cloud_color: COLORREF,
    pub(super) background: COLORREF,
    pub(super) highlight: COLORREF,
}

impl Palette {
    /// 贴近 mac light：label / secondary / tertiary label、systemOrange、systemTeal。
    pub(super) fn light() -> Self {
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
    pub(super) fn dark() -> Self {
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
