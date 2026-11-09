//! 同步时屏幕边缘的霓虹灯：每台显示器四条贴边的透明窗口，点得穿、不抢焦点。只有传输时才有这个线程在画。
//! 只开四条窄窗口：整屏透明窗口每帧要算几百万像素，还容易挡住别的程序的点击。
//! 这里管节奏与颜色，[`shade`] 算像素；开窗口、贴像素、全屏与动画开关怎么判断在各平台的 `neon_strips`。

mod shade;

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use super::platform::{Strips, animations_on, should_hide};

pub(super) use shade::{Edge, PixelOrder};

const FRAME: Duration = Duration::from_millis(33);

/// 流动至少这么久才收，文字一瞬间传完也看得见。
const MIN_FLOW: Duration = Duration::from_millis(600);

const CLOSE_TIME: Duration = Duration::from_millis(550);

/// 失败收得慢一点，留出闪两下的时间。
const FAIL_CLOSE_TIME: Duration = Duration::from_millis(900);

/// 传输卡住太久没有新消息就自己收掉，灯不能一直亮着。
const STALL_LIMIT: Duration = Duration::from_secs(20);

const SLOW_BYTES: u64 = 1024 * 1024;

const FAST_BYTES: u64 = 10 * 1024 * 1024;

pub(super) const BLUE: [f32; 3] = [64.0, 140.0, 255.0];

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
        strips.draw(color, if animated { elapsed } else { 0.0 }, strength);
    }
}

/// 转成结果的颜色，再收起。
fn close(strips: &Strips, from: [f32; 3], success: bool, time: f32, animated: bool) {
    let (result, duration) = if success {
        (GREEN, CLOSE_TIME)
    } else {
        (RED, FAIL_CLOSE_TIME)
    };
    let started = Instant::now();
    loop {
        let progress = started.elapsed().as_secs_f32() / duration.as_secs_f32();
        if progress >= 1.0 {
            return;
        }
        // 失败要在第一下闪之前就红透
        let turn = if success { 3.0 } else { 6.0 };
        let color = mix(from, result, (progress * turn).min(1.0));
        let moving = if animated {
            time + started.elapsed().as_secs_f32()
        } else {
            0.0
        };
        strips.draw(color, moving, closing_strength(progress, success));
        std::thread::sleep(FRAME);
    }
}

/// 收起时的亮度（`progress` 0..1）。成功平稳淡出；失败先闪两下再收：
/// 红绿色弱的人分不清转绿和转红，靠闪不闪也看得出出了事。
fn closing_strength(progress: f32, success: bool) -> f32 {
    let fade = 1.0 - ((progress - 0.5).max(0.0) * 2.0).powi(2);
    if success {
        return fade;
    }
    // 先暗下去，0.25 与 0.75 亮、0.5 暗：红色亮两下
    let flash = 0.5 - 0.5 * (progress * 2.0 * std::f32::consts::TAU).cos();
    fade * flash
}

#[cfg(test)]
mod tests {
    use super::{BLUE, CYAN, VIOLET, closing_strength, speed_color};

    #[test]
    fn color_follows_speed() {
        assert_eq!(speed_color(0), BLUE);
        assert_eq!(speed_color(2 * 1024 * 1024), CYAN);
        assert_eq!(speed_color(50 * 1024 * 1024), VIOLET);
    }

    #[test]
    fn failure_flashes_twice_and_success_fades() {
        let steps: Vec<f32> = (0..=20).map(|step| step as f32 / 20.0).collect();
        let success: Vec<f32> = steps.iter().map(|&p| closing_strength(p, true)).collect();
        assert!(success.windows(2).all(|pair| pair[1] <= pair[0]));
        let failure = |p: f32| closing_strength(p, false);
        assert!(failure(0.0) < 0.01 && failure(0.5) < 0.01 && failure(1.0) < 0.01);
        assert!(failure(0.25) > 0.9 && failure(0.75) > 0.6);
    }
}
