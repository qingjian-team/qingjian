//! 主题元信息访问器：id / 显示名 / 作者 / 许可证、锁定外观、动画开关与键盘圆角。

use super::file::{LockedAppearance, ThemeFile};
use super::Theme;

impl Theme {
    pub(crate) fn animations(&self) -> bool {
        self.animations
    }

    /// 手机键盘每个键的圆角（主题 `keyboard.touch_key_radius`，点）；主题没写是 `None`，手机壳用自己的缺省。
    pub fn touch_key_radius(&self) -> Option<f32> {
        self.file.keyboard.metrics.touch_key_radius
    }

    /// 手机键盘整块的圆角（主题 `keyboard.touch_radius`，点）；主题没写是 `None`。
    pub fn touch_radius(&self) -> Option<f32> {
        self.file.keyboard.metrics.touch_radius
    }

    /// 主题 id。
    pub fn id(&self) -> &str {
        &self.file.meta.id
    }

    /// 显示名。
    pub fn name(&self) -> &str {
        &self.file.meta.name
    }

    pub fn author(&self) -> &str {
        &self.file.meta.author
    }

    /// SPDX 许可证标识。
    pub fn license(&self) -> &str {
        &self.file.meta.license
    }

    /// 锁定的外观：`Some(true)` 只有深色，`Some(false)` 只有浅色。
    pub(crate) fn locked_dark(&self) -> Option<bool> {
        self.file
            .meta
            .appearance
            .map(|appearance| appearance == LockedAppearance::Dark)
    }

    pub(crate) fn file(&self) -> &ThemeFile {
        &self.file
    }
}
