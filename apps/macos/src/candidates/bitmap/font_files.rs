//! 按字族名找系统里的字体文件（CoreText 的字体登记），给渲染器只加载这几个文件；再列出可选的字族名给设置页。

use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::NSFontManager;
use objc2_core_foundation::{CFArray, CFDictionary, CFRetained, CFString, CFURL, CFURLPathStyle};
use objc2_core_text::{CTFontDescriptor, kCTFontFamilyNameAttribute, kCTFontURLAttribute};

/// 某字族所有面的文件路径，去重；系统里没有这个字族返回空。
pub(crate) fn family_files(family: &str) -> Vec<PathBuf> {
    if family.trim().is_empty() {
        return Vec::new();
    }
    // SAFETY: 只读 CoreText 导出的属性名常量；属性字典键值都是 CFString，与 CoreText 的约定一致
    let descriptors = unsafe {
        let key: &CFString = kCTFontFamilyNameAttribute;
        let value = CFString::from_str(family);
        let attributes = CFDictionary::from_slices(&[key], &[&*value]);
        let descriptor = CTFontDescriptor::with_attributes(attributes.as_opaque());
        descriptor.matching_font_descriptors(None)
    };
    let mut files: Vec<PathBuf> = Vec::new();
    let Some(descriptors) = descriptors else {
        return files;
    };
    // SAFETY: CTFontDescriptorCreateMatchingFontDescriptors 返回的数组元素就是 CTFontDescriptor
    let descriptors: CFRetained<CFArray<CTFontDescriptor>> =
        unsafe { CFRetained::cast_unchecked(descriptors) };
    for descriptor in descriptors.iter() {
        // SAFETY: 只读导出的常量名
        let url = unsafe { descriptor.attribute(kCTFontURLAttribute) }
            .and_then(|value| value.downcast::<CFURL>().ok());
        let Some(path) =
            url.and_then(|url| url.file_system_path(CFURLPathStyle::CFURLPOSIXPathStyle))
        else {
            continue;
        };
        let path = PathBuf::from(path.to_string());
        if !files.contains(&path) {
            files.push(path);
        }
    }
    files
}

/// 系统里可选的字族名，按系统给的顺序。
pub(crate) fn available_families(mtm: MainThreadMarker) -> Vec<String> {
    NSFontManager::sharedFontManager(mtm)
        .availableFontFamilies()
        .iter()
        .map(|name| name.to_string())
        // 以点开头的是系统私有字体，用户选不着也不该列
        .filter(|name| !name.starts_with('.'))
        .collect()
}
