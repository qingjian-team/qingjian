//! 候选窗口的点击区域：候选本体与它右侧的各条译词各占一块，壳按鼠标落点查是哪一块。

mod panel;
mod preedit;
mod region;
mod target;

pub use panel::PanelHit;
pub use preedit::PreeditHit;
pub use region::HitRegion;
pub use target::HitTarget;
