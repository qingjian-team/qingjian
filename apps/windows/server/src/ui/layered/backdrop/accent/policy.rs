//! `ACCENT_POLICY`：要哪种强调效果、着色。

#[repr(C)]
pub(super) struct AccentPolicy {
    /// `ACCENT_*` 之一。
    pub(super) state: u32,

    pub(super) flags: u32,

    /// 着色，`0xAABBGGRR`。
    pub(super) gradient_color: u32,

    pub(super) animation_id: u32,
}
