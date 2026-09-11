//! 造 [`TextService`] 的 COM 类厂。[`DllGetClassObject`](super::DllGetClassObject) 把它交给系统，
//! 系统再调 `CreateInstance` 造出真正的文本服务对象。

use core::ffi::c_void;

use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_FAIL};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::core::{BOOL, GUID, IUnknown, Interface, Ref, Result, implement};

use super::service::TextService;

/// 无状态类厂。用 `#[implement]` 让它成为一个 COM 对象（实现 `IClassFactory`）。
#[implement(IClassFactory)]
pub struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Ref<IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        // 不支持聚合。
        if !punkouter.is_null() {
            return CLASS_E_NOAGGREGATION.ok();
        }
        if riid.is_null() || ppvobject.is_null() {
            return E_FAIL.ok();
        }
        let unknown: IUnknown = TextService::new().into();
        // SAFETY: riid 指向有效 GUID，ppvobject 可写一个接口指针。
        unsafe { unknown.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            super::lock_module();
        } else {
            super::unlock_module();
        }
        Ok(())
    }
}
