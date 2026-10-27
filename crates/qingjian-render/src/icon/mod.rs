//! 主题里能写的矢量小图标（`"icon": "cloud"`），各自实现引擎的 [`bamboo_slip::Icon`]。

mod chevron;
mod cloud;
mod gear;

use std::sync::{Arc, LazyLock};

use serde::Deserialize;

use chevron::Chevron;
use cloud::Cloud;
use gear::Gear;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Icon {
    /// 云联想的云朵。
    Cloud,

    /// 状态条打开设置的齿轮。
    Gear,

    /// 翻页箭头：上一页。
    ChevronLeft,

    /// 翻页箭头：下一页。
    ChevronRight,
}

static CLOUD: LazyLock<Arc<dyn bamboo_slip::Icon>> = LazyLock::new(|| Arc::new(Cloud));

static GEAR: LazyLock<Arc<dyn bamboo_slip::Icon>> = LazyLock::new(|| Arc::new(Gear));

static CHEVRON_LEFT: LazyLock<Arc<dyn bamboo_slip::Icon>> =
    LazyLock::new(|| Arc::new(Chevron { right: false }));

static CHEVRON_RIGHT: LazyLock<Arc<dyn bamboo_slip::Icon>> =
    LazyLock::new(|| Arc::new(Chevron { right: true }));

impl Icon {
    /// 交给场景树画的那一个。
    pub(crate) fn painter(self) -> Arc<dyn bamboo_slip::Icon> {
        match self {
            Self::Cloud => Arc::clone(&CLOUD),
            Self::Gear => Arc::clone(&GEAR),
            Self::ChevronLeft => Arc::clone(&CHEVRON_LEFT),
            Self::ChevronRight => Arc::clone(&CHEVRON_RIGHT),
        }
    }
}
