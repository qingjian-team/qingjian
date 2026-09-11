//! 中 / 英输入模式指示器：Win11 托盘品牌图标左边的模式图标。按微软 IME 的做法经 `GUID_LBI_INPUTMODE`
//! 语言栏按钮把图标交给系统（转换模式 compartment 不走这条通道，光写它不显示）。

use std::rc::Rc;

use windows::Win32::Foundation::{COLORREF, E_NOINTERFACE, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateBitmap,
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DEFAULT_CHARSET, DIB_RGB_COLORS, DT_CENTER,
    DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, FF_DONTCARE, FW_NORMAL, GdiFlush,
    OUT_TT_PRECIS, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, VARIABLE_PITCH,
};
use windows::Win32::UI::TextServices::{
    GUID_LBI_INPUTMODE, ITfLangBarItem_Impl, ITfLangBarItemButton, ITfLangBarItemButton_Impl,
    ITfLangBarItemSink, ITfMenu, ITfSource, ITfSource_Impl, TF_LANGBARITEMINFO,
    TF_LBI_STYLE_BTN_BUTTON, TfLBIClick,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO};
use windows::core::{BOOL, BSTR, GUID, IUnknown, Interface, Ref, Result, implement, w};

use super::ModeState;
use crate::com::CLSID_QINGJIAN;

/// `GUID_LBI_INPUTMODE` 语言栏按钮：图标随 [`ModeState`] 显示中 / 英，点它切模式。
#[implement(ITfLangBarItemButton, ITfSource)]
pub(crate) struct ModeButton {
    state: Rc<ModeState>,
}

impl ModeButton {
    pub(crate) fn create(state: Rc<ModeState>) -> ITfLangBarItemButton {
        Self { state }.into()
    }
}

impl ITfLangBarItem_Impl for ModeButton_Impl {
    fn GetInfo(&self, pinfo: *mut TF_LANGBARITEMINFO) -> Result<()> {
        let info = unsafe { &mut *pinfo };
        info.clsidService = CLSID_QINGJIAN;
        info.guidItem = GUID_LBI_INPUTMODE;
        info.dwStyle = TF_LBI_STYLE_BTN_BUTTON;
        info.ulSort = 0;
        let desc: Vec<u16> = "青简中英模式".encode_utf16().collect();
        let n = desc.len().min(info.szDescription.len());
        info.szDescription[..n].copy_from_slice(&desc[..n]);
        Ok(())
    }

    fn GetStatus(&self) -> Result<u32> {
        Ok(0)
    }

    fn Show(&self, _fshow: BOOL) -> Result<()> {
        Ok(())
    }

    fn GetTooltipString(&self) -> Result<BSTR> {
        Ok(BSTR::from("中 / 英（单击 Shift 切换）"))
    }
}

impl ITfLangBarItemButton_Impl for ModeButton_Impl {
    fn OnClick(&self, _click: TfLBIClick, _pt: &POINT, _prcarea: *const RECT) -> Result<()> {
        crate::com::service::toggle_mode();
        Ok(())
    }

    fn InitMenu(&self, _pmenu: Ref<ITfMenu>) -> Result<()> {
        Ok(())
    }

    fn OnMenuSelect(&self, _wid: u32) -> Result<()> {
        Ok(())
    }

    fn GetIcon(&self) -> Result<HICON> {
        make_mode_icon(if self.state.english() { '英' } else { '中' })
    }

    fn GetText(&self) -> Result<BSTR> {
        Ok(BSTR::from(if self.state.english() { "英" } else { "中" }))
    }
}

impl ITfSource_Impl for ModeButton_Impl {
    fn AdviseSink(&self, riid: *const GUID, punk: Ref<IUnknown>) -> Result<u32> {
        if unsafe { *riid } != ITfLangBarItemSink::IID {
            return Err(E_NOINTERFACE.into());
        }
        let sink: ITfLangBarItemSink = punk.ok()?.cast()?;
        *self.state.sink.borrow_mut() = Some(sink);
        Ok(1) // 只支持一个回调，cookie 固定
    }

    fn UnadviseSink(&self, _dwcookie: u32) -> Result<()> {
        *self.state.sink.borrow_mut() = None;
        Ok(())
    }
}

/// 透明底、白字的「中」/「英」图标（Win11 深色托盘可见）。画得比托盘尺寸大，系统缩小后更锐。
/// 系统取走 HICON 后负责销毁。
fn make_mode_icon(ch: char) -> Result<HICON> {
    const SIZE: i32 = 24;
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: SIZE,
            biHeight: -SIZE, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = core::ptr::null_mut();
    unsafe {
        let color = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)?;
        let memdc = CreateCompatibleDC(None);
        let old_bmp = SelectObject(memdc, color.into());
        let font = CreateFontW(
            -(SIZE - 2), // 负字高 = 精确字符高度
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_TT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            (VARIABLE_PITCH.0 | FF_DONTCARE.0) as u32,
            w!("Microsoft YaHei UI"),
        );
        let old_font = SelectObject(memdc, font.into());
        let _ = SetBkMode(memdc, TRANSPARENT);
        SetTextColor(memdc, COLORREF(0x00FF_FFFF));
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: SIZE,
            bottom: SIZE,
        };
        let mut text: Vec<u16> = ch.to_string().encode_utf16().collect();
        DrawTextW(
            memdc,
            &mut text,
            &mut rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        let _ = GdiFlush();
        // GDI 画字不写 alpha：画上字的像素补成不透明，其余保持透明。
        let pixels = std::slice::from_raw_parts_mut(bits.cast::<u32>(), (SIZE * SIZE) as usize);
        for p in pixels.iter_mut().filter(|p| **p & 0x00FF_FFFF != 0) {
            *p |= 0xFF00_0000;
        }
        SelectObject(memdc, old_font);
        let _ = DeleteObject(font.into());
        SelectObject(memdc, old_bmp);
        let _ = DeleteDC(memdc);
        // 掩码全 0，透明靠 32bpp 的 alpha。
        let mask = CreateBitmap(SIZE, SIZE, 1, 1, None);
        let info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        };
        let icon = CreateIconIndirect(&info);
        let _ = DeleteObject(mask.into());
        let _ = DeleteObject(color.into());
        icon
    }
}
