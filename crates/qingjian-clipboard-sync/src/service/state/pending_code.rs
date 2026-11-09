//! [`PendingCode`]：这台正在显示的配对码。

use std::time::Instant;

pub(in crate::service) struct PendingCode {
    pub code: String,

    pub expires: Instant,
}
