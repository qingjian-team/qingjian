//! 主题的 `material`：窗口底下垫哪种系统材质，垫上之后窗口底色换成什么着色，文字深浅跟谁。

use serde::Deserialize;

use super::MaterialAppearance;
use crate::Material;
use crate::theme::file::ColorSpec;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MaterialSpec {
    pub(crate) kind: Material,

    /// 盖在材质上的着色，代替窗口根节点的 `fill`；不写就不着色。壳垫不了材质时照旧用 `fill`。
    pub(crate) tint: Option<ColorSpec>,

    /// `"auto"`：文字跟着材质随背后内容反色；`"system"`：跟系统外观，材质也钉在系统外观上。
    /// 不写时 `glass` 是 `auto`（新玻璃钉不住外观，不跟就只能退回毛玻璃），`blur` 是 `system`。
    pub(crate) appearance: Option<MaterialAppearance>,
}

impl MaterialSpec {
    pub(crate) fn appearance(&self) -> MaterialAppearance {
        self.appearance.unwrap_or(match self.kind {
            Material::Glass => MaterialAppearance::Auto,
            Material::Blur => MaterialAppearance::System,
        })
    }
}
