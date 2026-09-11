//! 组句 preedit 的内联下划线：按 TSF 显示属性协议给组句范围标一个「输入中」属性，宿主应用据此在拼音底下
//! 画下划线（对应 macOS marked text 的下划线）。
//!
//! 三部分：(1) 一个自定义显示属性 GUID + 它的 [`TF_DISPLAYATTRIBUTE`]（细实线下划线，前景 / 背景不改）；
//! (2) 系统按 `GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER` 类别经 `ITfDisplayAttributeProvider`（实现在
//! [`super::service::TextService`] 上）来取属性描述，这里放它要交回的 [`AttributeInfo`] / [`AttributeEnum`]
//! 两个 COM 对象；(3) 收键写组句时把 GUID 经类别管理器换成 atom，写进组句范围的 `GUID_PROP_ATTRIBUTE` 属性。

use core::cell::Cell;

use windows::Win32::Foundation::S_FALSE;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, GUID_PROP_ATTRIBUTE, IEnumTfDisplayAttributeInfo,
    IEnumTfDisplayAttributeInfo_Impl, ITfCategoryMgr, ITfContext, ITfDisplayAttributeInfo,
    ITfDisplayAttributeInfo_Impl, ITfRange, TF_ATTR_INPUT, TF_CT_NONE, TF_DA_COLOR,
    TF_DISPLAYATTRIBUTE, TF_LS_DOT,
};
use windows::core::{BSTR, Error, GUID, Result, implement};

use super::log::log;
use super::variant::i4;

/// 青简的组句显示属性 GUID（自定义）。注册进类别管理器换成 atom，与注册表里声明的显示属性提供者类别配套。
pub(crate) const GUID_DISPLAY_ATTRIBUTE_INPUT: GUID =
    GUID::from_u128(0xc47cb4c0_0ac9_4c8f_bdbf_8b6d21cc504f);

thread_local! {
    /// [`GUID_DISPLAY_ATTRIBUTE_INPUT`] 在类别管理器里的 atom：进程内稳定，首次用时算出来缓存。0 = 还没算。
    static ATOM: Cell<u32> = const { Cell::new(0) };
}

/// 「输入中」的显示属性：点虚线下划线（IME 组句惯例样式），前景 / 背景 / 线色都不指定（跟随文档），
/// 对应 macOS 组句下划线。想要更长的破折线可换 `TF_LS_DASH`。
fn input_attribute() -> TF_DISPLAYATTRIBUTE {
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

/// 给组句范围打上内联下划线属性。失败只记日志（不影响文字本身）。
pub(crate) fn mark(context: &ITfContext, ec: u32, range: &ITfRange) {
    if let Err(error) = mark_inner(context, ec, range) {
        log(&format!("组句下划线属性写入失败: {error}"));
    }
}

fn mark_inner(context: &ITfContext, ec: u32, range: &ITfRange) -> Result<()> {
    let variant = i4(guid_atom()? as i32);
    // SAFETY: GUID / ec / range 均有效；SetValue 只读 variant。
    let property = unsafe { context.GetProperty(&GUID_PROP_ATTRIBUTE)? };
    unsafe { property.SetValue(ec, range, &variant) }
}

/// 取（并缓存）显示属性 GUID 对应的 atom。
fn guid_atom() -> Result<u32> {
    let cached = ATOM.with(Cell::get);
    if cached != 0 {
        return Ok(cached);
    }
    // SAFETY: CLSID 有效；组句写入在 STA UI 线程上，COM 已初始化。
    let manager: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
    // SAFETY: GUID 指针有效。
    let atom = unsafe { manager.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE_INPUT)? };
    ATOM.with(|a| a.set(atom));
    Ok(atom)
}

/// 交给系统的显示属性枚举器（青简只有这一个属性）。
pub(crate) fn enumerator() -> IEnumTfDisplayAttributeInfo {
    AttributeEnum {
        done: Cell::new(false),
    }
    .into()
}

/// 交给系统的显示属性描述对象。
pub(crate) fn info() -> ITfDisplayAttributeInfo {
    AttributeInfo.into()
}

/// 青简组句显示属性的描述：GUID + [`TF_DISPLAYATTRIBUTE`]，系统据此渲染组句样式。用户不可改样式。
#[implement(ITfDisplayAttributeInfo)]
struct AttributeInfo;

impl ITfDisplayAttributeInfo_Impl for AttributeInfo_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(GUID_DISPLAY_ATTRIBUTE_INPUT)
    }

    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("青简拼音"))
    }

    fn GetAttributeInfo(&self, pda: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        // SAFETY: 系统传入的可写指针。
        unsafe { *pda = input_attribute() };
        Ok(())
    }

    fn SetAttributeInfo(&self, _pda: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        // 不支持用户自定义样式：收下但不改。
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        Ok(())
    }
}

/// 只含一个 [`AttributeInfo`] 的枚举器；`done` 记这唯一的属性是否已发出。
#[implement(IEnumTfDisplayAttributeInfo)]
struct AttributeEnum {
    /// 唯一的属性是否已被取走。
    done: Cell<bool>,
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
            // SAFETY: rginfo 指向至少 ulcount 个槽。
            unsafe { *rginfo = Some(info()) };
            self.done.set(true);
            fetched = 1;
        }
        if !pcfetched.is_null() {
            // SAFETY: 调用方给的可写指针。
            unsafe { *pcfetched = fetched };
        }
        // 取满返回 S_OK，不足返回 S_FALSE（枚举器约定）。
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
