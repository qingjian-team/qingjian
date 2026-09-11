//! 自注册：写 COM 的 InprocServer32，并经 TSF 的 `ITfInputProcessorProfiles` / `ITfCategoryMgr`
//! 把青简登记成一个键盘类文本服务。regsvr32（通常要管理员）调 [`register`] / [`unregister`]。
//!
//! 写的是 `HKEY_CLASSES_ROOT`（机器级），所以 regsvr32 需管理员权限。

use core::ptr;

use windows::Win32::Foundation::{ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CLASSES_ROOT, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_COMLESS, GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT, GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIPCAP_UIELEMENTENABLED, ITfCategoryMgr,
    ITfInputProcessorProfiles,
};
use windows::core::{GUID, HRESULT, HSTRING, PCWSTR, Result};

/// 要登记的 TSF 类别。除了「键盘类 TIP」，还必须声明沉浸式 / 系统托盘等能力，否则 Windows 10/11 的
/// 现代输入切换器会把这个 TIP 过滤掉（表现为「装上又消失」）。与微软 SampleIME 的一组一致。
const CATEGORIES: &[GUID] = &[
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_COMLESS,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
];

/// 写 InprocServer32 + 注册 TSF 文本服务 / 语言 profile / 键盘类别。
pub(crate) fn register() -> Result<()> {
    let module = super::module_path()?;
    let base = format!("CLSID\\{}", super::CLSID_QINGJIAN_STR);
    let inproc = format!("{base}\\InprocServer32");

    // SAFETY: 各字符串以 0 结尾，键路径合法。
    unsafe {
        set_value(&base, None, super::SERVICE_DESCRIPTION)?;
        set_value(&inproc, None, &module.to_string())?;
        set_value(&inproc, Some("ThreadingModel"), "Apartment")?;
    }

    register_profile()
}

/// 撤销 [`register`]：先撤 TSF 登记，再删 CLSID 注册表子树。尽力而为，单步失败不挡后续。
pub(crate) fn unregister() -> Result<()> {
    let _ = unregister_profile();
    let base = format!("CLSID\\{}", super::CLSID_QINGJIAN_STR);
    let base_w = HSTRING::from(base);
    // SAFETY: base_w 以 0 结尾。删整棵子树。
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, &base_w);
    }
    Ok(())
}

/// 向 TSF 注册文本服务、加中文语言 profile、登记为键盘类别（这样它出现在系统输入法列表里）。
fn register_profile() -> Result<()> {
    com_scope(|| {
        // SAFETY: CLSID 有效；APARTMENTTHREADED 已初始化。
        let profiles: ITfInputProcessorProfiles = unsafe {
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?
        };
        // 描述串必须以 0 结尾：AddLanguageProfile 的实现按 null 扫描读它，不加 0 会多读进相邻内存
        // 的一个字符（曾表现为「青简C」）。图标文件同理给一个 null。
        let mut description: Vec<u16> = super::SERVICE_DESCRIPTION.encode_utf16().collect();
        description.push(0);
        // SAFETY: 各指针指向有效 GUID / 以 0 结尾的切片。
        unsafe {
            profiles.Register(&super::CLSID_QINGJIAN)?;
            profiles.AddLanguageProfile(
                &super::CLSID_QINGJIAN,
                super::LANGID_ZH_CN,
                &super::GUID_PROFILE,
                &description,
                &[0], // 图标文件：暂无（一个 null）
                0,
            )?;
        }

        // SAFETY: CLSID 有效。
        let category: ITfCategoryMgr =
            unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
        for catid in CATEGORIES {
            // SAFETY: 三个 GUID 指针有效。登记为键盘 TIP 并声明各项能力。
            unsafe {
                category.RegisterCategory(&super::CLSID_QINGJIAN, catid, &super::CLSID_QINGJIAN)?;
            }
        }
        Ok(())
    })
}

/// 撤销 [`register_profile`]。尽力而为。
fn unregister_profile() -> Result<()> {
    com_scope(|| {
        // SAFETY: CLSID 有效。
        if let Ok(category) = unsafe {
            CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)
        } {
            for catid in CATEGORIES {
                // SAFETY: 三个 GUID 指针有效。
                unsafe {
                    let _ = category.UnregisterCategory(
                        &super::CLSID_QINGJIAN,
                        catid,
                        &super::CLSID_QINGJIAN,
                    );
                }
            }
        }
        // SAFETY: CLSID 有效。
        if let Ok(profiles) = unsafe {
            CoCreateInstance::<_, ITfInputProcessorProfiles>(
                &CLSID_TF_InputProcessorProfiles,
                None,
                CLSCTX_INPROC_SERVER,
            )
        } {
            // SAFETY: GUID 指针有效。
            unsafe {
                let _ = profiles.RemoveLanguageProfile(
                    &super::CLSID_QINGJIAN,
                    super::LANGID_ZH_CN,
                    &super::GUID_PROFILE,
                );
                let _ = profiles.Unregister(&super::CLSID_QINGJIAN);
            }
        }
        Ok(())
    })
}

/// 在一段 COM 初始化 / 反初始化之间跑闭包。regsvr32 一般已初始化过 COM，这里配对调用只为稳妥；
/// 已是别的套间模式（`RPC_E_CHANGED_MODE`）就不重复反初始化。
fn com_scope<T>(body: impl FnOnce() -> Result<T>) -> Result<T> {
    // SAFETY: 标准 COM 初始化。
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let initialized = hr.is_ok();
    let result = body();
    if initialized {
        // SAFETY: 与上面的 CoInitializeEx 配对。
        unsafe { CoUninitialize() };
    }
    result
}

/// 建（或打开）`HKEY_CLASSES_ROOT\<subkey>`，写一个 `REG_SZ` 值（`name` 为 `None` 写默认值）。
///
/// # Safety
/// `subkey` / `name` / `data` 以有效 UTF-8 给出，函数内部转宽字符并加结尾 0。
unsafe fn set_value(subkey: &str, name: Option<&str>, data: &str) -> Result<()> {
    let subkey_w = HSTRING::from(subkey);
    let mut hkey = HKEY(ptr::null_mut());
    // SAFETY: 参数均有效，phkresult 可写。
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CLASSES_ROOT,
            &subkey_w,
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    win32_ok(status)?;

    // 值内容：宽字符 + 结尾 0，按字节写。
    let mut wide: Vec<u16> = data.encode_utf16().collect();
    wide.push(0);
    // SAFETY: wide 有 wide.len() 个 u16，按 *2 字节读。
    let bytes = unsafe { core::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2) };

    let result = match name {
        Some(name) => {
            let name_w = HSTRING::from(name);
            // SAFETY: hkey 有效，name_w 以 0 结尾，bytes 可读。
            win32_ok(unsafe { RegSetValueExW(hkey, &name_w, None, REG_SZ, Some(bytes)) })
        }
        // SAFETY: hkey 有效，默认值名传 null，bytes 可读。
        None => {
            win32_ok(unsafe { RegSetValueExW(hkey, PCWSTR::null(), None, REG_SZ, Some(bytes)) })
        }
    };

    // SAFETY: hkey 由 RegCreateKeyExW 得来，用完关闭。
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    result
}

/// 把 Win32 错误码转成 `Result`。
fn win32_ok(status: WIN32_ERROR) -> Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(HRESULT::from_win32(status.0).into())
    }
}
