//! [`Pairing`]：设备页里正在做的配对。

use bamboo_slip::TextEdit;

/// 设备页里正在做的配对。
pub enum Pairing {
    Idle,

    /// 这台显示配对码。
    Showing(String),

    /// 这台输入对方的地址和配对码。
    Entering {
        address: TextEdit,
        code: TextEdit,
    },
}
