//! 状态条的一格。

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::HFONT;

use super::placement::StatusAction;

/// 一格：文字、字体、颜色、点下去做什么。
pub(super) struct CellSpec {
    pub(super) text: String,

    pub(super) font: HFONT,

    pub(super) color: COLORREF,

    pub(super) action: StatusAction,
}
