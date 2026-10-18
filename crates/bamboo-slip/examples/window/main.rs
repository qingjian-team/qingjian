//! 窗口宿主样例：一页能点、能滚的设置页（竹青配色），跟随 DPI 与深浅色。要开 `host` 特性。
//!
//! - `cargo run --release --features host --example window`：开窗口。侧栏能切页、控件能点能拨，Tab / Shift+Tab 走焦点、空格触发，Esc 退出。
//! - `cargo run --release --features host --example window -- --png <目录>`：不开窗口，按脚本发事件，每步存一张截图，
//!   外加一张总览 `sheet.png` 与开关动画的逐帧图 `film.png`。

mod demo;
mod film;
mod icons;
mod page;
mod palette;
mod snapshot;

use std::path::PathBuf;

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{App, RenderError, Ui, WindowOptions, run_window};

use demo::{Demo, Message};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--png") {
        let directory = args.next().map(PathBuf::from).ok_or("--png 后面要给目录")?;
        snapshot::run(&directory)?;
        return film::run(&directory);
    }
    run_window(
        Demo::new(false),
        WindowOptions::new("bamboo-slip", (780.0, 580.0)),
    )?;
    Ok(())
}

impl App for Demo {
    type Message = Message;

    fn view(
        &self,
        ui: &mut Ui<Message>,
        scale: f32,
        size: (u32, u32),
    ) -> Result<NodeId, RenderError> {
        page::view(ui, self, scale, size)
    }

    fn update(&mut self, message: Message) -> bool {
        Demo::update(self, message)
    }

    fn set_system_dark(&mut self, dark: bool) {
        self.system_dark = dark;
    }
}
