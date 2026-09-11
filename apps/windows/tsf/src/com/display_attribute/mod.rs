//! 组句 preedit 的内联下划线：按 TSF 显示属性协议给组句范围标一个「输入中」属性（对应 macOS marked text 的下划线）。
//! 系统经 `ITfDisplayAttributeProvider`（实现在 [`super::service::TextService`]）来取 [`AttributeInfo`]
//! （枚举器 [`AttributeEnum`]）；写组句时把 GUID 换成 atom 写进范围的 `GUID_PROP_ATTRIBUTE`。

mod enumerator;
mod info;

use core::cell::Cell;

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, GUID_PROP_ATTRIBUTE, IEnumTfDisplayAttributeInfo, ITfCategoryMgr,
    ITfContext, ITfDisplayAttributeInfo, ITfRange, TF_ATTR_INPUT, TF_CT_NONE, TF_DA_COLOR,
    TF_DISPLAYATTRIBUTE, TF_LS_DOT,
};
use windows::core::{GUID, Result};

use self::enumerator::AttributeEnum;
use self::info::AttributeInfo;
use super::log::log;

/// 青简的组句显示属性 GUID（自定义），与注册表里声明的显示属性提供者类别配套。
pub(crate) const GUID_DISPLAY_ATTRIBUTE_INPUT: GUID =
    GUID::from_u128(0xc47cb4c0_0ac9_4c8f_bdbf_8b6d21cc504f);

thread_local! {
    /// GUID 在类别管理器里的 atom，首次用时算出来缓存；0 = 还没算。
    static ATOM: Cell<u32> = const { Cell::new(0) };
}

/// 点虚线下划线，前景 / 背景 / 线色跟随文档。
pub(super) fn input_attribute() -> TF_DISPLAYATTRIBUTE {
    let follow_text = TF_DA_COLOR {
        r#type: TF_CT_NONE,
        ..Default::default()
    };
    TF_DISPLAYATTRIBUTE {
        crText: follow_text,
        crBk: follow_text,
        lsStyle: TF_LS_DOT,
        fBoldLine: false.into(),
        crLine: follow_text,
        bAttr: TF_ATTR_INPUT,
    }
}

/// 给组句范围打上下划线属性。失败只记日志。
pub(crate) fn mark(context: &ITfContext, ec: u32, range: &ITfRange) {
    if let Err(error) = mark_inner(context, ec, range) {
        log(&format!("组句下划线属性写入失败: {error}"));
    }
}

fn mark_inner(context: &ITfContext, ec: u32, range: &ITfRange) -> Result<()> {
    let variant = VARIANT::from(guid_atom()? as i32);
    let property = unsafe { context.GetProperty(&GUID_PROP_ATTRIBUTE)? };
    unsafe { property.SetValue(ec, range, &variant) }
}

fn guid_atom() -> Result<u32> {
    let cached = ATOM.with(Cell::get);
    if cached != 0 {
        return Ok(cached);
    }
    let manager: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
    let atom = unsafe { manager.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE_INPUT)? };
    ATOM.with(|a| a.set(atom));
    Ok(atom)
}

pub(crate) fn enumerator() -> IEnumTfDisplayAttributeInfo {
    AttributeEnum {
        done: Cell::new(false),
    }
    .into()
}

pub(crate) fn info() -> ITfDisplayAttributeInfo {
    AttributeInfo.into()
}
