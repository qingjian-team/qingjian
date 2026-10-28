//! 受控退出的退出码与固定原因标识。

pub(crate) struct ExitStatus {
    pub(crate) code: u8,

    pub(crate) reason: &'static str,
}

impl ExitStatus {
    pub(crate) const fn new(code: u8, reason: &'static str) -> Self {
        Self { code, reason }
    }
}
