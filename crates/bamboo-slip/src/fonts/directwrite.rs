//! Windows 的字体登记（DirectWrite）：按字族名找出字体文件交给 [`crate::FontLibrary`] 只加载这几个，再列出全部字族名（字体选择界面用）。

use std::path::PathBuf;

use ::windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection,
    IDWriteFontFile, IDWriteLocalFontFileLoader, IDWriteLocalizedStrings,
};
use ::windows::core::{BOOL, HSTRING, Interface};

/// 系统字体集合；DirectWrite 不可用时为 `None`。
fn system_collection() -> Option<IDWriteFontCollection> {
    // SAFETY: 共享工厂随进程存活；出参按文档传指针。
    unsafe {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).ok()?;
        let mut collection = None;
        factory
            .GetSystemFontCollection(&mut collection, false)
            .ok()?;
        collection
    }
}

/// 某字族所有面的文件路径，去重；系统里没有这个字族返回空。
pub fn family_files(family: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    let Some(collection) = system_collection() else {
        return files;
    };
    // SAFETY: 都是 DirectWrite 的只读查询；出参缓冲按它报的长度分配。
    unsafe {
        let mut index = 0u32;
        let mut exists = BOOL(0);
        if collection
            .FindFamilyName(&HSTRING::from(family), &mut index, &mut exists)
            .is_err()
            || !exists.as_bool()
        {
            return files;
        }
        let Ok(family) = collection.GetFontFamily(index) else {
            return files;
        };
        for i in 0..family.GetFontCount() {
            let Ok(face) = family.GetFont(i).and_then(|font| font.CreateFontFace()) else {
                continue;
            };
            let mut count = 0u32;
            if face.GetFiles(&mut count, None).is_err() || count == 0 {
                continue;
            }
            let mut handles: Vec<Option<IDWriteFontFile>> = vec![None; count as usize];
            if face
                .GetFiles(&mut count, Some(handles.as_mut_ptr()))
                .is_err()
            {
                continue;
            }
            for file in handles.into_iter().flatten() {
                if let Some(path) = file_path(&file)
                    && !files.contains(&path)
                {
                    files.push(path);
                }
            }
        }
    }
    files
}

/// 本机字体文件的路径；网络 / 内存字体（不是本地加载器）返回 `None`。
fn file_path(file: &IDWriteFontFile) -> Option<PathBuf> {
    // SAFETY: 引用键由 DirectWrite 持有、随 file 存活；路径缓冲按报的长度加终止符分配。
    unsafe {
        let mut key = std::ptr::null_mut();
        let mut size = 0u32;
        file.GetReferenceKey(&mut key, &mut size).ok()?;
        let loader: IDWriteLocalFontFileLoader = file.GetLoader().ok()?.cast().ok()?;
        let len = loader.GetFilePathLengthFromKey(key, size).ok()?;
        let mut buffer = vec![0u16; len as usize + 1];
        loader.GetFilePathFromKey(key, size, &mut buffer).ok()?;
        buffer.truncate(len as usize);
        Some(PathBuf::from(String::from_utf16_lossy(&buffer)))
    }
}

/// 系统里全部字族名（英文名，没有英文名取第一个），按名字排序。
pub fn families() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let Some(collection) = system_collection() else {
        return names;
    };
    // SAFETY: 只读查询。
    unsafe {
        for i in 0..collection.GetFontFamilyCount() {
            let name = collection
                .GetFontFamily(i)
                .ok()
                .and_then(|family| family.GetFamilyNames().ok())
                .and_then(|strings| localized(&strings));
            if let Some(name) = name
                && !names.contains(&name)
            {
                names.push(name);
            }
        }
    }
    names.sort_by_key(|name| name.to_lowercase());
    names
}

/// 本地化字符串里的英文名，没有就取第 0 条。
fn localized(strings: &IDWriteLocalizedStrings) -> Option<String> {
    // SAFETY: 只读查询；缓冲按报的长度加终止符分配。
    unsafe {
        let mut index = 0u32;
        let mut exists = BOOL(0);
        let _ = strings.FindLocaleName(&HSTRING::from("en-us"), &mut index, &mut exists);
        if !exists.as_bool() {
            index = 0;
        }
        let len = strings.GetStringLength(index).ok()?;
        let mut buffer = vec![0u16; len as usize + 1];
        strings.GetString(index, &mut buffer).ok()?;
        buffer.truncate(len as usize);
        Some(String::from_utf16_lossy(&buffer))
    }
}
