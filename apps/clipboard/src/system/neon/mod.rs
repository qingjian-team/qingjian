//! 同步时屏幕边缘的霓虹灯：每台显示器四条贴边的透明窗口，点得穿、不抢焦点。只有传输时才有这个线程在画。
//! 只开四条窄窗口：整屏透明窗口每帧要算几百万像素，还容易挡住别的程序的点击。

mod strip;
mod strips;

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};
use windows::core::BOOL;

use strips::Strips;

const FRAME: Duration = Duration::from_millis(33);

/// 流动至少这么久才收，文字一瞬间传完也看得见。
const MIN_FLOW: Duration = Duration::from_millis(600);

const CLOSE_TIME: Duration = Duration::from_millis(550);

/// 传输卡住太久没有新消息就自己收掉，灯不能一直亮着。
const STALL_LIMIT: Duration = Duration::from_secs(20);

const SLOW_BYTES: u64 = 1024 * 1024;

const FAST_BYTES: u64 = 10 * 1024 * 1024;

const BLUE: [f32; 3] = [64.0, 140.0, 255.0];

const CYAN: [f32; 3] = [40.0, 220.0, 235.0];

const VIOLET: [f32; 3] = [175.0, 95.0, 255.0];

const GREEN: [f32; 3] = [55.0, 225.0, 120.0];

const RED: [f32; 3] = [255.0, 70.0, 70.0];

enum Command {
    Start,
    Speed(u64),
    Finish { success: bool },
}

/// 灯的遥控器，发完就走，不等动画。
#[derive(Clone)]
pub struct Neon(Sender<Command>);

impl Neon {
    pub fn spawn() -> Self {
        let (sender, receiver) = channel();
        std::thread::Builder::new()
            .name("clipboard-neon".to_owned())
            .spawn(move || run(receiver))
            .map_err(|error| tracing::warn!(%error, "cannot start neon thread"))
            .ok();
        Self(sender)
    }

    pub fn start(&self) {
        let _ = self.0.send(Command::Start);
    }

    pub fn speed(&self, bytes_per_second: u64) {
        let _ = self.0.send(Command::Speed(bytes_per_second));
    }

    pub fn finish(&self, success: bool) {
        let _ = self.0.send(Command::Finish { success });
    }
}

fn speed_color(bytes_per_second: u64) -> [f32; 3] {
    if bytes_per_second >= FAST_BYTES {
        VIOLET
    } else if bytes_per_second >= SLOW_BYTES {
        CYAN
    } else {
        BLUE
    }
}

fn mix(from: [f32; 3], to: [f32; 3], amount: f32) -> [f32; 3] {
    std::array::from_fn(|index| from[index] + (to[index] - from[index]) * amount)
}

fn run(commands: Receiver<Command>) {
    while let Ok(command) = commands.recv() {
        match command {
            Command::Start if !should_hide() => animate(&commands),
            // 发送失败没有 Start，直接亮一下红色告诉用户
            Command::Finish { success: false } if !should_hide() => {
                let strips = Strips::create();
                close(&strips, BLUE, false, 0.0, false);
            }
            _ => {}
        }
    }
}

fn animate(commands: &Receiver<Command>) {
    let strips = Strips::create();
    let animated = animations_on();
    let started = Instant::now();
    let mut heard = started;
    let mut target = BLUE;
    let mut color = BLUE;
    let mut result = None;
    loop {
        match commands.recv_timeout(FRAME) {
            Ok(Command::Start) => heard = Instant::now(),
            Ok(Command::Speed(speed)) => {
                heard = Instant::now();
                if speed > 0 {
                    target = speed_color(speed);
                }
            }
            Ok(Command::Finish { success }) => result = Some(success),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let elapsed = started.elapsed().as_secs_f32();
        if result.is_none() && heard.elapsed() > STALL_LIMIT {
            result = Some(false);
        }
        if let Some(success) = result
            && started.elapsed() >= MIN_FLOW
        {
            close(&strips, color, success, elapsed, animated);
            return;
        }
        color = mix(color, target, 0.12);
        // 刚亮起时淡入
        let strength = (elapsed / 0.2).min(1.0);
        strips.draw(color, if animated { elapsed } else { 0.0 }, strength, None);
    }
}

/// 转成结果的颜色，再收起。
fn close(strips: &Strips, from: [f32; 3], success: bool, time: f32, animated: bool) {
    let result = if success { GREEN } else { RED };
    let started = Instant::now();
    loop {
        let progress = started.elapsed().as_secs_f32() / CLOSE_TIME.as_secs_f32();
        if progress >= 1.0 {
            return;
        }
        let color = mix(from, result, (progress * 3.0).min(1.0));
        let strength = 1.0 - ((progress - 0.5).max(0.0) * 2.0).powi(2);
        let moving = if animated {
            time + started.elapsed().as_secs_f32()
        } else {
            0.0
        };
        strips.draw(color, moving, strength, Some((success, strength)));
        std::thread::sleep(FRAME);
    }
}

/// 全屏游戏、演示时不打扰。
fn should_hide() -> bool {
    let Ok(state) = (unsafe { SHQueryUserNotificationState() }) else {
        return false;
    };
    state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE || state == QUNS_BUSY
}

/// 系统「显示动画」关了就不流动，只亮和变色。
fn animations_on() -> bool {
    let mut on = BOOL(1);
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut on).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_err() || on.as_bool()
}

#[cfg(test)]
mod tests {
    use super::{BLUE, CYAN, VIOLET, speed_color};

    #[test]
    fn color_follows_speed() {
        assert_eq!(speed_color(0), BLUE);
        assert_eq!(speed_color(2 * 1024 * 1024), CYAN);
        assert_eq!(speed_color(50 * 1024 * 1024), VIOLET);
    }
}
