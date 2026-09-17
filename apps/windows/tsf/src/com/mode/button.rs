//! 中 / 英输入模式指示器：Win11 托盘品牌图标左边的模式图标。按微软 IME 的做法经 `GUID_LBI_INPUTMODE`
//! 语言栏按钮把图标交给系统（转换模式 compartment 不走这条通道，光写它不显示）。Caps Lock 亮着显示「A」。

use std::rc::Rc;

use windows::Win32::Foundation::{E_NOINTERFACE, POINT, RECT};
use windows::Win32::UI::TextServices::{
    GUID_LBI_INPUTMODE, ITfLangBarItem_Impl, ITfLangBarItemButton, ITfLangBarItemButton_Impl,
    ITfLangBarItemSink, ITfMenu, ITfSource, ITfSource_Impl, TF_LANGBARITEMINFO,
    TF_LBI_STYLE_BTN_BUTTON, TfLBIClick,
};
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::core::{BOOL, BSTR, GUID, IUnknown, Interface, Ref, Result, implement};

use super::ModeState;
use super::icon::{self, Glyph};
use crate::com::CLSID_QINGJIAN;
use crate::com::key::event::caps_lock_on;

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
        icon::make(self.glyph())
    }

    fn GetText(&self) -> Result<BSTR> {
        Ok(BSTR::from(match self.glyph() {
            Glyph::Chinese => "中",
            Glyph::English => "英",
            Glyph::CapsLock => "A",
        }))
    }
}

impl ModeButton_Impl {
    /// Caps 亮着无论中英模式都直接出大写英文，所以它优先。
    fn glyph(&self) -> Glyph {
        if caps_lock_on() {
            Glyph::CapsLock
        } else if self.state.english() {
            Glyph::English
        } else {
            Glyph::Chinese
        }
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
