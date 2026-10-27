//! 交给应用的候选列表元素：应用经 `ITfUIElementMgr::GetUIElement` 拿到它，读当前页的候选与高亮。

use std::cell::{Cell, RefCell};

use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG};
use windows::Win32::UI::TextServices::{
    ITfCandidateListUIElement, ITfCandidateListUIElement_Impl, ITfDocumentMgr, ITfUIElement_Impl,
    TF_CLUIE_COUNT, TF_CLUIE_CURRENTPAGE, TF_CLUIE_DOCUMENTMGR, TF_CLUIE_PAGEINDEX,
    TF_CLUIE_SELECTION, TF_CLUIE_STRING,
};
use windows::core::{BOOL, BSTR, GUID, Result, implement};

use qingjian_platform::protocol::Frame;

/// 这类 UI 元素的 GUID（`ITfUIElement::GetGUID`）。
const GUID_CANDIDATE_LIST: GUID = GUID::from_u128(0x3db2ddc0_ce7a_49a1_a29f_79de15f52eb5);

/// 一段组句的候选列表。线上帧只带当前页，所以对应用来说整个列表就是这一页：只有一页、当前是第 0 页。
#[implement(ITfCandidateListUIElement)]
pub(crate) struct CandidateElement {
    items: RefCell<Vec<String>>,

    /// 高亮的候选在 `items` 里的下标。
    selection: Cell<u32>,

    /// 输入法要不要自己画候选：`BeginUIElement` 回的 `bShow`，之后应用还能用 `Show` 改。
    shown: Cell<bool>,

    document: Option<ITfDocumentMgr>,
}

impl CandidateElement {
    pub(crate) fn new(document: Option<ITfDocumentMgr>) -> Self {
        Self {
            items: RefCell::default(),
            selection: Cell::new(0),
            shown: Cell::new(true),
            document,
        }
    }

    pub(crate) fn shown(&self) -> bool {
        self.shown.get()
    }

    pub(crate) fn set_shown(&self, shown: bool) {
        self.shown.set(shown);
    }

    /// 换成这一帧的当前页；没变返回 `false`（轮询每拍都回帧，没变不必让应用重读）。
    pub(crate) fn set_frame(&self, frame: &Frame) -> bool {
        let items: Vec<String> = frame
            .candidates
            .items
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect();
        // 拼音行有焦点时帧里没有高亮（`usize::MAX`），给应用的下标仍要落在列表里
        let selection = if frame.highlight < items.len() {
            frame.highlight as u32
        } else {
            0
        };
        if *self.items.borrow() == items && self.selection.get() == selection {
            return false;
        }
        *self.items.borrow_mut() = items;
        self.selection.set(selection);
        true
    }
}

impl ITfUIElement_Impl for CandidateElement_Impl {
    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("青简候选"))
    }

    fn GetGUID(&self) -> Result<GUID> {
        Ok(GUID_CANDIDATE_LIST)
    }

    fn Show(&self, bshow: BOOL) -> Result<()> {
        self.shown.set(bshow.as_bool());
        Ok(())
    }

    fn IsShown(&self) -> Result<BOOL> {
        Ok(self.shown.get().into())
    }
}

impl ITfCandidateListUIElement_Impl for CandidateElement_Impl {
    fn GetUpdatedFlags(&self) -> Result<u32> {
        Ok(TF_CLUIE_DOCUMENTMGR
            | TF_CLUIE_COUNT
            | TF_CLUIE_SELECTION
            | TF_CLUIE_STRING
            | TF_CLUIE_PAGEINDEX
            | TF_CLUIE_CURRENTPAGE)
    }

    fn GetDocumentMgr(&self) -> Result<ITfDocumentMgr> {
        self.document.clone().ok_or_else(|| E_FAIL.into())
    }

    fn GetCount(&self) -> Result<u32> {
        Ok(self.items.borrow().len() as u32)
    }

    fn GetSelection(&self) -> Result<u32> {
        Ok(self.selection.get())
    }

    fn GetString(&self, uindex: u32) -> Result<BSTR> {
        self.items
            .borrow()
            .get(uindex as usize)
            .map(|text| BSTR::from(text.as_str()))
            .ok_or_else(|| E_INVALIDARG.into())
    }

    fn GetPageIndex(&self, pindex: *mut u32, usize: u32, pupagecnt: *mut u32) -> Result<()> {
        unsafe {
            *pupagecnt = 1;
            if !pindex.is_null() && usize > 0 {
                *pindex = 0;
            }
        }
        Ok(())
    }

    /// 翻页按青简的按键走，不接受应用改分页。
    fn SetPageIndex(&self, _pindex: *const u32, _upagecnt: u32) -> Result<()> {
        Ok(())
    }

    fn GetCurrentPage(&self) -> Result<u32> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::{Candidate, CandidateKind, CandidateList};

    fn frame(texts: &[&str], highlight: usize) -> Frame {
        let items = texts
            .iter()
            .map(|text| Candidate {
                text: text.to_string(),
                kind: CandidateKind::Chinese,
                syllables: Vec::new(),
                reading: None,
                translation: None,
                aux_code: None,
            })
            .collect();
        Frame {
            candidates: CandidateList { items },
            highlight,
            ..Frame::default()
        }
    }

    #[test]
    fn set_frame_reports_changes_and_keeps_selection_in_list() {
        let element = CandidateElement::new(None);
        assert!(element.set_frame(&frame(&["你", "尼"], 1)));
        assert_eq!(element.selection.get(), 1);
        // 轮询回的同一帧不让应用重读
        assert!(!element.set_frame(&frame(&["你", "尼"], 1)));
        // 拼音行有焦点：帧里没有高亮
        assert!(element.set_frame(&frame(&["你", "尼"], usize::MAX)));
        assert_eq!(element.selection.get(), 0);
        assert_eq!(*element.items.borrow(), ["你", "尼"]);
    }
}
