//! 候选窗的系统材质：按每帧的 [`MaterialBacking`] 给窗口开 / 关亚克力与 DWM 圆角。

use std::cell::Cell;

use qingjian_render::MaterialBacking;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DWM_WINDOW_CORNER_PREFERENCE, DWMWA_BORDER_COLOR, DWMWA_COLOR_DEFAULT, DWMWA_COLOR_NONE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DEFAULT, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::core::Result;

use super::accent::{self, ACCENT_ACRYLIC, ACCENT_DISABLED};
use super::transparency_enabled;

/// 浅色亚克力的着色（`0xAABBGGRR`）：近白、约七成不透明，深色字压在任何背景上都读得清。
const LIGHT_TINT: u32 = 0xB0F8_F6F6;

/// 深色亚克力的着色：深灰、约七成不透明。
const DARK_TINT: u32 = 0xB02E_2C2C;

pub(in crate::ui) struct Backdrop {
    hwnd: HWND,

    /// 系统垫得了：DWM 认窗口圆角（Windows 11 起）且有亚克力；垫失败过一次就当垫不了。
    capable: Cell<bool>,

    /// 正垫着的亚克力是不是深色；`None` 没垫。
    applied: Cell<Option<bool>>,
}

impl Backdrop {
    /// 探测系统能不能垫：Windows 10 不认 `DWMWA_WINDOW_CORNER_PREFERENCE`，设缺省值就会失败。
    pub(in crate::ui) fn new(hwnd: HWND) -> Self {
        let corner = set_corner(hwnd, DWMWCP_DEFAULT);
        let capable = accent::available() && corner.is_ok();
        tracing::info!(
            capable,
            corner = ?corner.err(),
            "candidate window material backdrop probed"
        );
        Self {
            hwnd,
            capable: Cell::new(capable),
            applied: Cell::new(None),
        }
    }

    /// 这一帧能不能垫：系统支持且用户没关透明效果。每次画候选前问一次，渲染器据此决定画不画不透明底。
    pub(in crate::ui) fn supported(&self) -> bool {
        self.capable.get() && transparency_enabled()
    }

    /// 按这一帧垫或撤材质。`dark` 是候选窗这一帧用的外观；主题锁定了外观时听主题的。
    /// 系统跟不了背后内容的深浅（`follows_backdrop`），文字照系统外观走，着色跟着换深浅。
    pub(in crate::ui) fn update(&self, material: Option<MaterialBacking>, dark: bool) {
        let wanted = material.map(|backing| backing.locked_dark.unwrap_or(dark));
        if wanted == self.applied.get() {
            return;
        }
        self.applied.set(wanted);
        let result = match wanted {
            Some(dark) => self.apply(dark),
            None => self.remove(),
        };
        if let Err(error) = result {
            tracing::warn!(%error, ?wanted, "failed to update candidate window backdrop");
            // 这一帧已按有材质画成透明底：撤干净，之后都画不透明底
            if wanted.is_some() {
                self.capable.set(false);
                self.applied.set(None);
                let _ = self.remove();
            }
        }
    }

    fn apply(&self, dark: bool) -> Result<()> {
        set_corner(self.hwnd, DWMWCP_ROUND)?;
        // 边框交给主题的描边，DWM 自己的那道灰边与之重叠会显得发虚
        set_border(self.hwnd, DWMWA_COLOR_NONE)?;
        let tint = if dark { DARK_TINT } else { LIGHT_TINT };
        if !accent::set(self.hwnd, ACCENT_ACRYLIC, tint) {
            return Err(windows::core::Error::from_thread());
        }
        Ok(())
    }

    fn remove(&self) -> Result<()> {
        accent::set(self.hwnd, ACCENT_DISABLED, 0);
        set_border(self.hwnd, DWMWA_COLOR_DEFAULT)?;
        set_corner(self.hwnd, DWMWCP_DEFAULT)
    }
}

fn set_corner(hwnd: HWND, preference: DWM_WINDOW_CORNER_PREFERENCE) -> Result<()> {
    // SAFETY: 传进去的是栈上 4 字节的枚举值
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&raw const preference).cast(),
            size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        )
    }
}

fn set_border(hwnd: HWND, color: u32) -> Result<()> {
    // SAFETY: 同上，4 字节 COLORREF
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            (&raw const color).cast(),
            size_of::<u32>() as u32,
        )
    }
}
