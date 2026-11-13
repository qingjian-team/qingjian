//! 系统材质：主题写了 `material` 时，候选窗底下由系统铺一层实时模糊的亚克力，渲染器只画上面的内容。
//!
//! 只在 Windows 11 上垫：分层窗口的模糊不认窗口区域，圆角只能交给 DWM 的窗口圆角（定死约 8 点，随 DPI 缩放），
//! 阴影也由 DWM 按圆角画；Windows 10 没有 DWM 圆角，垫上就是直角模糊块，所以照旧画主题的不透明底与投影。
//! 用户在系统设置里关了「透明效果」时也不垫。取舍见 `docs/design/theme.md` 的「系统材质」。

mod accent;
mod window;

pub(in crate::ui) use window::Backdrop;

/// DWM 给窗口裁的圆角（点）：Windows 11 的 `DWMWCP_ROUND`。渲染器按它画根节点，描边与系统裁的形状对齐。
pub(in crate::ui) const CORNER_RADIUS: f32 = 8.0;

/// 系统设置「个性化 → 颜色 → 透明效果」开着（`EnableTransparency` 不为 0；读不到当开着）。
fn transparency_enabled() -> bool {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_u32("EnableTransparency"))
        .ok()
        .is_none_or(|value| value != 0)
}
