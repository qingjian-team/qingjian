//! 只含一个 [`AttributeInfo`](super::info::AttributeInfo) 的显示属性枚举器。

use core::cell::Cell;

use windows::Win32::Foundation::S_FALSE;
use windows::Win32::UI::TextServices::{
    IEnumTfDisplayAttributeInfo, IEnumTfDisplayAttributeInfo_Impl, ITfDisplayAttributeInfo,
};
use windows::core::{Error, Result, implement};

use super::info;

/// 只含一个 [`AttributeInfo`](super::info::AttributeInfo) 的枚举器。
#[implement(IEnumTfDisplayAttributeInfo)]
pub(super) struct AttributeEnum {
    /// 唯一的属性是否已被取走。
    pub(super) done: Cell<bool>,
}

impl IEnumTfDisplayAttributeInfo_Impl for AttributeEnum_Impl {
    fn Clone(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(AttributeEnum {
            done: Cell::new(self.done.get()),
        }
        .into())
    }

    fn Next(
        &self,
        ulcount: u32,
        rginfo: *mut Option<ITfDisplayAttributeInfo>,
        pcfetched: *mut u32,
    ) -> Result<()> {
        let mut fetched = 0u32;
        if ulcount >= 1 && !self.done.get() {
            unsafe { *rginfo = Some(info()) };
            self.done.set(true);
            fetched = 1;
        }
        if !pcfetched.is_null() {
            unsafe { *pcfetched = fetched };
        }
        // 枚举器约定：取满 S_OK，不足 S_FALSE。
        if fetched == ulcount {
            Ok(())
        } else {
            Err(Error::from(S_FALSE))
        }
    }

    fn Reset(&self) -> Result<()> {
        self.done.set(false);
        Ok(())
    }

    fn Skip(&self, ulcount: u32) -> Result<()> {
        if ulcount >= 1 {
            self.done.set(true);
        }
        Ok(())
    }
}
