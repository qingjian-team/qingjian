//! 状态条的摆放状态与点击分发：窗口过程按 HWND 查到它，拖动结束记位置、点击按格分发动作（[`StatusAction`]）。

mod action;

use std::cell::{Cell, RefCell};

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

pub(super) use self::action::StatusAction;
use crate::dispatch::StatusEvent;
use crate::ui::StatusEvents;

/// 窗口过程也要读写的摆放状态，经 `PLACEMENTS` 按 HWND 查到。
pub(super) struct Placement {
    /// 状态条窗口。
    pub(super) hwnd: HWND,

    /// 上次合成用的阴影留白，拖动结束时从窗口矩形反推内容左上角用；点击时把客户区坐标换成内容坐标。
    pub(super) margin: Cell<i32>,

    /// 内容左上角的屏幕坐标（物理像素）；`None` 表示还没摆放过。
    pub(super) pos: Cell<Option<(i32, i32)>>,

    /// 上次画出的各格右边界（内容坐标）与动作，从左到右；点击按 x 落进哪格。
    pub(super) cells: RefCell<Vec<(i32, StatusAction)>>,

    /// 点格 / 拖动结束回给 Router。
    events: StatusEvents,
}

impl Placement {
    pub(super) fn new(hwnd: HWND, margin: i32, events: StatusEvents) -> Self {
        Self {
            hwnd,
            margin: Cell::new(margin),
            pos: Cell::new(None),
            cells: RefCell::new(Vec::new()),
            events,
        }
    }

    /// 拖动结束：从窗口矩形反推内容左上角，记下并交给 Router 写回配置。
    pub(super) fn on_moved(&self) {
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut rect) }.is_ok() {
            let margin = self.margin.get();
            let x = rect.left + margin;
            let y = rect.top + margin;
            self.pos.set(Some((x, y)));
            (self.events)(StatusEvent::Moved(x, y));
        }
    }

    /// 单击：`client_x` 是客户区横坐标。
    pub(super) fn on_click(&self, client_x: i32) {
        let x = client_x - self.margin.get();
        let action = self
            .cells
            .borrow()
            .iter()
            .find(|(right, _)| x < *right)
            .map(|(_, action)| *action);
        match action {
            Some(StatusAction::ToggleMode) => (self.events)(StatusEvent::ToggleMode),
            Some(StatusAction::TogglePunctuation) => {
                (self.events)(StatusEvent::TogglePunctuation);
            }
            Some(StatusAction::OpenSettings) => open_settings(),
            None => {}
        }
    }
}

/// 起与本 exe 同目录的设置程序。
fn open_settings() {
    let exe = std::env::current_exe().map(|exe| exe.with_file_name("qingjian-settings.exe"));
    let spawned = exe.and_then(|exe| std::process::Command::new(exe).spawn());
    if let Err(error) = spawned {
        tracing::warn!(%error, "打开设置程序失败");
    }
}
