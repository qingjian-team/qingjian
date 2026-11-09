//! 通过真实 COM vtable 验证未初始化输出槽与非法指针，不操作宿主输入框。

use std::mem::MaybeUninit;
use std::ptr::null_mut;

use windows::Win32::Foundation::{E_POINTER, S_FALSE, S_OK};
use windows::Win32::UI::TextServices::{ITfDisplayAttributeInfo, ITfDisplayAttributeProvider};
use windows::core::Interface;

#[test]
fn enumeration_initializes_native_output_and_reports_exhaustion() {
    let enumerator = super::enumerator();
    let mut slot = MaybeUninit::<Option<ITfDisplayAttributeInfo>>::uninit();
    let mut fetched = 0;
    let next = Interface::vtable(&enumerator).Next;
    let first = unsafe {
        next(
            enumerator.as_raw(),
            1,
            slot.as_mut_ptr().cast(),
            &mut fetched,
        )
    };
    assert_eq!(first, S_OK);
    assert_eq!(fetched, 1);
    let value = unsafe { slot.assume_init() }.unwrap();
    assert_eq!(
        unsafe { value.GetGUID() }.unwrap(),
        super::GUID_DISPLAY_ATTRIBUTE_INPUT
    );
    let mut exhausted_slot = MaybeUninit::<Option<ITfDisplayAttributeInfo>>::uninit();
    let exhausted = unsafe {
        next(
            enumerator.as_raw(),
            1,
            exhausted_slot.as_mut_ptr().cast(),
            &mut fetched,
        )
    };
    assert_eq!(exhausted, S_FALSE);
    assert_eq!(fetched, 0);
    // 第二次没有产出，不能读取未初始化的输出槽。
}

#[test]
fn invalid_output_pointers_return_hresult_instead_of_panicking() {
    let enumerator = super::enumerator();
    let next = Interface::vtable(&enumerator).Next;
    let mut fetched = 0;
    assert_eq!(
        unsafe { next(enumerator.as_raw(), 1, null_mut(), &mut fetched) },
        E_POINTER
    );
    let info = super::info();
    assert_eq!(
        unsafe { (Interface::vtable(&info).GetAttributeInfo)(info.as_raw(), null_mut()) },
        E_POINTER
    );
}

#[test]
fn fetched_count_is_optional_for_zero_single_and_multiple_requests() {
    for count in [0, 1, 2] {
        let enumerator = super::enumerator();
        let mut slots = MaybeUninit::<[Option<ITfDisplayAttributeInfo>; 2]>::uninit();
        let result = unsafe {
            (Interface::vtable(&enumerator).Next)(
                enumerator.as_raw(),
                count,
                slots.as_mut_ptr().cast(),
                null_mut(),
            )
        };
        assert_eq!(result, if count <= 1 { S_OK } else { S_FALSE });
        if count > 0 {
            // 只有第一项实际产出；第二项没有初始化，不能读或释放。
            let first = unsafe {
                slots
                    .as_ptr()
                    .cast::<Option<ITfDisplayAttributeInfo>>()
                    .read()
            };
            assert!(first.is_some());
        }
    }
}

#[test]
fn missing_attribute_guid_returns_hresult_instead_of_panicking() {
    let provider: ITfDisplayAttributeProvider = crate::com::service::TextService::new().into();
    let mut output = null_mut();
    let result = unsafe {
        (Interface::vtable(&provider).GetDisplayAttributeInfo)(
            provider.as_raw(),
            std::ptr::null(),
            &mut output,
        )
    };
    assert_eq!(result, E_POINTER);
    assert!(output.is_null());
}
