//! 自注册：写 COM 的 InprocServer32，经 `ITfInputProcessorProfiles` / `ITfCategoryMgr` 把青简登记成键盘类文本服务。
//! 写的是 `HKEY_CLASSES_ROOT`，所以 regsvr32 要管理员。输入法图标文件在 [`icon`]。

mod icon;

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_TIP_KEYBOARD, GUID_TFCAT_TIPCAP_COMLESS, GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT, GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIPCAP_UIELEMENTENABLED, ITfCategoryMgr,
    ITfInputProcessorProfiles,
};
use windows::core::{GUID, Result};
use windows_registry::CLASSES_ROOT;

use crate::com::{
    CLSID_QINGJIAN, CLSID_QINGJIAN_STR, GUID_PROFILE, LANGID_ZH_CN, SERVICE_DESCRIPTION,
};

/// 除「键盘类 TIP」外还要声明沉浸式 / 系统托盘等能力，否则 Win10/11 的输入切换器会把它过滤掉（表现为「装上又消失」）。
const CATEGORIES: &[GUID] = &[
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_COMLESS,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
];

fn clsid_key() -> String {
    format!("CLSID\\{CLSID_QINGJIAN_STR}")
}

pub(crate) fn register() -> Result<()> {
    let module = crate::com::module_path()?;
    let base = CLASSES_ROOT.create(clsid_key())?;
    base.set_string("", SERVICE_DESCRIPTION)?;
    let inproc = base.create("InprocServer32")?;
    inproc.set_string("", module.to_string())?;
    inproc.set_string("ThreadingModel", "Apartment")?;
    register_profile()
}

/// 撤销 [`register`]，尽力而为。
pub(crate) fn unregister() -> Result<()> {
    let _ = unregister_profile();
    icon::uninstall();
    let _ = CLASSES_ROOT.remove_tree(clsid_key());
    Ok(())
}

fn register_profile() -> Result<()> {
    com_scope(|| {
        let profiles: ITfInputProcessorProfiles = unsafe {
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?
        };
        // AddLanguageProfile 按 null 扫描读字符串，不补 0 会多读相邻内存（曾显示成「青简C」）。
        let description = wide_z(SERVICE_DESCRIPTION);
        let icon = icon::install()
            .map(|path| wide_z(&path.to_string_lossy()))
            .unwrap_or_else(|| vec![0]);
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
        let category: ITfCategoryMgr =
            unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
        for catid in CATEGORIES {
            unsafe { category.RegisterCategory(&CLSID_QINGJIAN, catid, &CLSID_QINGJIAN)? };
        }
        Ok(())
    })
}

fn unregister_profile() -> Result<()> {
    com_scope(|| {
        if let Ok(category) = unsafe {
            CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)
        } {
            for catid in CATEGORIES {
                let _ =
                    unsafe { category.UnregisterCategory(&CLSID_QINGJIAN, catid, &CLSID_QINGJIAN) };
            }
        }
        if let Ok(profiles) = unsafe {
            CoCreateInstance::<_, ITfInputProcessorProfiles>(
                &CLSID_TF_InputProcessorProfiles,
                None,
                CLSCTX_INPROC_SERVER,
            )
        } {
            unsafe {
                let _ =
                    profiles.RemoveLanguageProfile(&CLSID_QINGJIAN, LANGID_ZH_CN, &GUID_PROFILE);
                let _ = profiles.Unregister(&CLSID_QINGJIAN);
            }
        }
        Ok(())
    })
}

/// regsvr32 一般已初始化过 COM；已是别的套间模式（`RPC_E_CHANGED_MODE`）就不配对反初始化。
fn com_scope<T>(body: impl FnOnce() -> Result<T>) -> Result<T> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    let result = body();
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

/// 以 0 结尾的宽字符串。
fn wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
