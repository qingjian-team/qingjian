//! AppKit 字体与补充描边：字族没有粗体面时仍能独立加粗。

use std::ops::Deref;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSFont, NSFontManager, NSFontTraitMask};
use objc2_foundation::NSString;

pub(crate) struct NativeFont {
    pub font: Retained<NSFont>,

    pub synthetic_bold: bool,
}

impl NativeFont {
    pub fn system(size: f64) -> Self {
        Self {
            font: NSFont::systemFontOfSize(size),
            synthetic_bold: false,
        }
    }

    pub fn selected(mtm: MainThreadMarker, family: &str, size: u16, bold: bool) -> Self {
        let size = f64::from(size);
        let manager = NSFontManager::sharedFontManager(mtm);
        let regular = (!family.is_empty())
            .then(|| {
                manager.fontWithFamily_traits_weight_size(
                    &NSString::from_str(family),
                    NSFontTraitMask::empty(),
                    5,
                    size,
                )
            })
            .flatten();
        let font = match (regular, bold) {
            (Some(font), true) => {
                manager.convertFont_toHaveTrait(&font, NSFontTraitMask::BoldFontMask)
            }
            (Some(font), false) => font,
            (None, true) => NSFont::boldSystemFontOfSize(size),
            (None, false) => NSFont::systemFontOfSize(size),
        };
        let synthetic_bold = bold
            && !manager
                .traitsOfFont(&font)
                .contains(NSFontTraitMask::BoldFontMask);
        Self {
            font,
            synthetic_bold,
        }
    }
}

impl Deref for NativeFont {
    type Target = NSFont;

    fn deref(&self) -> &Self::Target {
        &self.font
    }
}
