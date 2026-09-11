//! 青简唯一的显示属性描述（组句「输入中」的点虚线下划线）；用户不可改样式。

use windows::Win32::UI::TextServices::{
    ITfDisplayAttributeInfo, ITfDisplayAttributeInfo_Impl, TF_DISPLAYATTRIBUTE,
};
use windows::core::{BSTR, GUID, Result, implement};

use super::{GUID_DISPLAY_ATTRIBUTE_INPUT, input_attribute};

/// 青简唯一的显示属性描述；用户不可改样式。
#[implement(ITfDisplayAttributeInfo)]
pub(super) struct AttributeInfo;

impl ITfDisplayAttributeInfo_Impl for AttributeInfo_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(GUID_DISPLAY_ATTRIBUTE_INPUT)
    }

    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("青简拼音"))
    }

    fn GetAttributeInfo(&self, pda: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        unsafe { *pda = input_attribute() };
        Ok(())
    }

    fn SetAttributeInfo(&self, _pda: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        Ok(())
    }
}
