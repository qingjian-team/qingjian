//! 自注册：写 COM 的 InprocServer32，并经 TSF 的 `ITfInputProcessorProfiles` / `ITfCategoryMgr` 把青简登记成
//! 键盘类文本服务。写的是 `HKEY_CLASSES_ROOT`，所以 regsvr32 要管理员。

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

use super::{CLSID_QINGJIAN, CLSID_QINGJIAN_STR, GUID_PROFILE, LANGID_ZH_CN, SERVICE_DESCRIPTION};

/// 要登记的 TSF 类别。除了「键盘类 TIP」还必须声明沉浸式 / 系统托盘等能力，否则 Win10/11 的现代输入切换器
/// 会把它过滤掉（表现为「装上又消失」）。与微软 SampleIME 一致。
const CATEGORIES: &[GUID] = &[
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_COMLESS,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
];

pub(crate) fn register() -> Result<()> {
    let module = super::module_path()?;
    let base = format!("CLSID\\{CLSID_QINGJIAN_STR}");
    let inproc = format!("{base}\\InprocServer32");
    set_value(&base, None, SERVICE_DESCRIPTION)?;
    set_value(&inproc, None, &module.to_string())?;
    set_value(&inproc, Some("ThreadingModel"), "Apartment")?;
    register_profile()
}

/// 撤销 [`register`]：先撤 TSF 登记，再删 CLSID 子树与图标文件。尽力而为。
pub(crate) fn unregister() -> Result<()> {
    let _ = unregister_profile();
    super::icon::uninstall();
    let base = HSTRING::from(format!("CLSID\\{CLSID_QINGJIAN_STR}"));
    // SAFETY: base 以 0 结尾。
    let _ = unsafe { RegDeleteTreeW(HKEY_CLASSES_ROOT, &base) };
    Ok(())
}

/// 注册文本服务、加中文语言 profile、登记类别（这样它出现在系统输入法列表里）。
fn register_profile() -> Result<()> {
    com_scope(|| {
        // SAFETY: CLSID 有效；COM 已初始化。
        let profiles: ITfInputProcessorProfiles = unsafe {
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?
        };
        // 描述串与图标路径都要以 0 结尾：AddLanguageProfile 按 null 扫描读，不加会多读相邻内存（曾显示成「青简C」）。
        let description = wide_z(SERVICE_DESCRIPTION);
        // 图标：写到机器级目录的 .ico，索引 0；写不了就无图标。
        let icon = super::icon::install()
            .map(|path| wide_z(&path.to_string_lossy()))
            .unwrap_or_else(|| vec![0]);
        // SAFETY: 各指针指向有效 GUID / 以 0 结尾的切片。
        unsafe {
            profiles.Register(&CLSID_QINGJIAN)?;
            profiles.AddLanguageProfile(
                &CLSID_QINGJIAN,
                LANGID_ZH_CN,
                &GUID_PROFILE,
                &description,
                &icon,
                0,
            )?;
        }

        // SAFETY: CLSID 有效。
        let category: ITfCategoryMgr =
            unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
        for catid in CATEGORIES {
            // SAFETY: 三个 GUID 指针有效。
            unsafe { category.RegisterCategory(&CLSID_QINGJIAN, catid, &CLSID_QINGJIAN)? };
        }
        Ok(())
    })
}

fn unregister_profile() -> Result<()> {
    com_scope(|| {
        // SAFETY: CLSID 有效。
        if let Ok(category) = unsafe {
            CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)
        } {
            for catid in CATEGORIES {
                // SAFETY: 三个 GUID 指针有效。
                let _ =
                    unsafe { category.UnregisterCategory(&CLSID_QINGJIAN, catid, &CLSID_QINGJIAN) };
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
                let _ =
                    profiles.RemoveLanguageProfile(&CLSID_QINGJIAN, LANGID_ZH_CN, &GUID_PROFILE);
                let _ = profiles.Unregister(&CLSID_QINGJIAN);
            }
        }
        Ok(())
    })
}

/// 在一段 COM 初始化 / 反初始化之间跑闭包。regsvr32 一般已初始化过 COM，配对调用只为稳妥；
/// 已是别的套间模式（`RPC_E_CHANGED_MODE`）就不重复反初始化。
fn com_scope<T>(body: impl FnOnce() -> Result<T>) -> Result<T> {
    // SAFETY: 标准 COM 初始化。
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    let result = body();
    if initialized {
        // SAFETY: 与上面的 CoInitializeEx 配对。
        unsafe { CoUninitialize() };
    }
    result
}

/// 建（或打开）`HKEY_CLASSES_ROOT\<subkey>`，写一个 `REG_SZ` 值（`name` 为 `None` 写默认值）。
fn set_value(subkey: &str, name: Option<&str>, data: &str) -> Result<()> {
    let subkey_w = HSTRING::from(subkey);
    let mut hkey = HKEY(ptr::null_mut());
    // SAFETY: 参数均有效，hkey 可写。
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

    // REG_SZ 按字节写：宽字符小端 + 结尾 0。
    let bytes: Vec<u8> = data
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    let name_w = name.map(HSTRING::from);
    let name_ptr = name_w
        .as_ref()
        .map_or(PCWSTR::null(), |n| PCWSTR(n.as_ptr()));
    // SAFETY: hkey 有效；name_ptr 为 null 或指向 name_w（活到函数结束）；bytes 可读。
    let result = win32_ok(unsafe { RegSetValueExW(hkey, name_ptr, None, REG_SZ, Some(&bytes)) });
    // SAFETY: hkey 由 RegCreateKeyExW 得来，用完关闭。
    let _ = unsafe { RegCloseKey(hkey) };
    result
}

/// 以 0 结尾的宽字符串。
fn wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn win32_ok(status: WIN32_ERROR) -> Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(HRESULT::from_win32(status.0).into())
    }
}
