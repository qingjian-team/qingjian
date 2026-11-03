//! 同步时屏幕边缘的霓虹灯：每台显示器四条贴边的透明窗口，点得穿、不抢焦点。只有传输时才有这个线程在画。
//! 只开四条窄窗口：整屏透明窗口每帧要算几百万像素，还容易挡住别的程序的点击。

use std::f32::consts::TAU;
use std::mem::size_of;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject,
    EnumDisplayMonitors, GetMonitorInfoW, HBITMAP, HDC, HMONITOR, MONITORINFO, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SPI_GETCLIENTAREAANIMATION,
    SW_SHOWNOACTIVATE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, ShowWindow, SystemParametersInfoW,
    ULW_ALPHA, UpdateLayeredWindow, WINDOW_EX_STYLE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{BOOL, w};

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

struct Strip {
    window: HWND,

    context: HDC,

    bitmap: HBITMAP,

    /// DIB 的像素：预乘的 BGRA，从上到下。
    pixels: *mut u8,

    origin: POINT,

    size: SIZE,

    side: Side,

    /// 所在显示器的宽高。
    monitor: (i32, i32),
}

struct Strips(Vec<Strip>);

impl Strips {
    fn create() -> Self {
        let mut strips = Vec::new();
        for monitor in monitors() {
            let width = monitor.right - monitor.left;
            let height = monitor.bottom - monitor.top;
            let thickness = (width.min(height) / 40).clamp(20, 64);
            let sides = [
                (Side::Top, monitor.left, monitor.top, width, thickness),
                (
                    Side::Bottom,
                    monitor.left,
                    monitor.bottom - thickness,
                    width,
                    thickness,
                ),
                (Side::Left, monitor.left, monitor.top, thickness, height),
                (
                    Side::Right,
                    monitor.right - thickness,
                    monitor.top,
                    thickness,
                    height,
                ),
            ];
            for (side, x, y, w, h) in sides {
                if let Some(strip) = Strip::create(side, (x, y), (w, h), (width, height)) {
                    strips.push(strip);
                }
            }
        }
        Self(strips)
    }

    fn draw(&self, color: [f32; 3], time: f32, strength: f32, mark: Option<(bool, f32)>) {
        for strip in &self.0 {
            strip.draw(color, time, strength, mark);
        }
    }
}

fn monitors() -> Vec<RECT> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        list: LPARAM,
    ) -> BOOL {
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..MONITORINFO::default()
        };
        // SAFETY: list 是下面传进来的 Vec，枚举期间一直活着。
        unsafe {
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                (*(list.0 as *mut Vec<RECT>)).push(info.rcMonitor);
            }
        }
        BOOL(1)
    }
    let mut list: Vec<RECT> = Vec::new();
    // SAFETY: 回调只在这次调用内用这个指针。
    let _ =
        unsafe { EnumDisplayMonitors(None, None, Some(collect), LPARAM((&raw mut list) as isize)) };
    list
}

unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: 原样转给系统的默认处理。
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

impl Strip {
    fn create(side: Side, at: (i32, i32), size: (i32, i32), monitor: (i32, i32)) -> Option<Self> {
        let instance = unsafe { GetModuleHandleW(None) }.ok()?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            lpszClassName: w!("QingjianClipboardNeon"),
            ..WNDCLASSW::default()
        };
        // 重复注册会失败，不要紧
        unsafe { RegisterClassW(&class) };
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(
                    WS_EX_LAYERED.0
                        | WS_EX_TRANSPARENT.0
                        | WS_EX_TOPMOST.0
                        | WS_EX_TOOLWINDOW.0
                        | WS_EX_NOACTIVATE.0,
                ),
                w!("QingjianClipboardNeon"),
                w!(""),
                WS_POPUP,
                at.0,
                at.1,
                size.0,
                size.1,
                None,
                None,
                Some(instance.into()),
                None,
            )
        }
        .ok()?;
        let context = unsafe { CreateCompatibleDC(None) };
        let header = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size.0,
            biHeight: -size.1,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..BITMAPINFOHEADER::default()
        };
        let info = BITMAPINFO {
            bmiHeader: header,
            ..BITMAPINFO::default()
        };
        let mut pixels = std::ptr::null_mut();
        let bitmap =
            unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut pixels, None, 0) }.ok()?;
        unsafe { SelectObject(context, bitmap.into()) };
        let strip = Self {
            window,
            context,
            bitmap,
            pixels: pixels.cast(),
            origin: POINT { x: at.0, y: at.1 },
            size: SIZE {
                cx: size.0,
                cy: size.1,
            },
            side,
            monitor,
        };
        strip.draw(BLUE, 0.0, 0.0, None);
        let _ = unsafe { ShowWindow(window, SW_SHOWNOACTIVATE) };
        Some(strip)
    }

    /// 像素沿屏幕一圈的位置（0..1）和离屏幕边的距离（像素）。
    fn locate(&self, x: i32, y: i32) -> (f32, i32) {
        let (width, height) = self.monitor;
        let perimeter = (2 * (width + height)) as f32;
        let (along, depth) = match self.side {
            Side::Top => (x, y),
            Side::Right => (width + y, self.size.cx - 1 - x),
            Side::Bottom => (width + height + (width - x), self.size.cy - 1 - y),
            Side::Left => (2 * width + height + (height - y), x),
        };
        (along as f32 / perimeter, depth)
    }

    fn draw(&self, color: [f32; 3], time: f32, strength: f32, mark: Option<(bool, f32)>) {
        let thickness = self.size.cx.min(self.size.cy) as f32;
        // SAFETY: pixels 是这条窗口自己的 DIB，大小就是 cx × cy × 4，只有这个线程写。
        let pixels = unsafe {
            std::slice::from_raw_parts_mut(self.pixels, (self.size.cx * self.size.cy * 4) as usize)
        };
        for y in 0..self.size.cy {
            for x in 0..self.size.cx {
                let (phase, depth) = self.locate(x, y);
                let fall = (1.0 - depth as f32 / thickness).max(0.0);
                let wave = 0.62 + 0.38 * ((phase * 3.0 - time * 0.7) * TAU).cos();
                let mut alpha = fall * fall * wave * strength;
                let mut shade = color;
                if let Some((success, opacity)) = mark {
                    let coverage = self.mark_coverage(x, y, success);
                    if coverage > 0.0 {
                        shade = mix(shade, [255.0, 255.0, 255.0], coverage);
                        alpha = alpha.max(coverage * opacity);
                    }
                }
                let offset = ((y * self.size.cx + x) * 4) as usize;
                // 预乘：UpdateLayeredWindow 要求
                pixels[offset] = (shade[2] * alpha) as u8;
                pixels[offset + 1] = (shade[1] * alpha) as u8;
                pixels[offset + 2] = (shade[0] * alpha) as u8;
                pixels[offset + 3] = (alpha * 255.0) as u8;
            }
        }
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
            ..BLENDFUNCTION::default()
        };
        let source = POINT::default();
        let _ = unsafe {
            UpdateLayeredWindow(
                self.window,
                None,
                Some(&self.origin),
                Some(&self.size),
                Some(self.context),
                Some(&source),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
        };
    }

    /// 顶边正中的 ✓ 或 ✕：色弱的人只靠红绿分不清结果。返回这个像素被盖住多少。
    fn mark_coverage(&self, x: i32, y: i32, success: bool) -> f32 {
        if self.side != Side::Top {
            return 0.0;
        }
        let height = self.size.cy as f32;
        let unit = height * 0.32;
        let center = (self.size.cx as f32 / 2.0, height * 0.5);
        let point = (x as f32 - center.0, y as f32 - center.1);
        let segments: [((f32, f32), (f32, f32)); 2] = if success {
            [
                ((-unit, 0.0), (-unit * 0.25, unit * 0.75)),
                ((-unit * 0.25, unit * 0.75), (unit, -unit * 0.75)),
            ]
        } else {
            [
                ((-unit * 0.8, -unit * 0.8), (unit * 0.8, unit * 0.8)),
                ((-unit * 0.8, unit * 0.8), (unit * 0.8, -unit * 0.8)),
            ]
        };
        let distance = segments
            .iter()
            .map(|(from, to)| segment_distance(point, *from, *to))
            .fold(f32::MAX, f32::min);
        let half_width = (height * 0.07).max(1.5);
        (half_width + 0.5 - distance).clamp(0.0, 1.0)
    }
}

fn segment_distance(point: (f32, f32), from: (f32, f32), to: (f32, f32)) -> f32 {
    let along = (to.0 - from.0, to.1 - from.1);
    let offset = (point.0 - from.0, point.1 - from.1);
    let length = along.0 * along.0 + along.1 * along.1;
    let t = if length > 0.0 {
        ((offset.0 * along.0 + offset.1 * along.1) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let gap = (offset.0 - along.0 * t, offset.1 - along.1 * t);
    (gap.0 * gap.0 + gap.1 * gap.1).sqrt()
}

impl Drop for Strip {
    fn drop(&mut self) {
        // SAFETY: 这些句柄都是这条窗口自己建的，只释放一次。
        unsafe {
            let _ = DestroyWindow(self.window);
            let _ = DeleteDC(self.context);
            let _ = DeleteObject(self.bitmap.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::Foundation::{HWND, POINT, SIZE};
    use windows::Win32::Graphics::Gdi::{HBITMAP, HDC};

    use super::{BLUE, CYAN, Side, Strip, VIOLET, segment_distance, speed_color};

    #[test]
    fn the_mark_covers_the_middle_of_the_top_edge() {
        let mut pixels = vec![0_u8; 1440 * 22 * 4];
        let strip = std::mem::ManuallyDrop::new(Strip {
            window: HWND::default(),
            context: HDC::default(),
            bitmap: HBITMAP::default(),
            pixels: pixels.as_mut_ptr(),
            origin: POINT::default(),
            size: SIZE { cx: 1440, cy: 22 },
            side: Side::Top,
            monitor: (1440, 900),
        });
        let covered = (0..22)
            .flat_map(|y| (700..740).map(move |x| (x, y)))
            .filter(|(x, y)| strip.mark_coverage(*x, *y, true) > 0.5)
            .count();
        assert!(covered > 20, "only {covered} pixels");
        assert_eq!(strip.mark_coverage(100, 11, true), 0.0);
        strip.draw(super::GREEN, 0.0, 1.0, Some((true, 1.0)));
        let white = pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200)
            .count();
        assert!(white > 20, "only {white} white pixels after drawing");
    }

    #[test]
    fn color_follows_speed() {
        assert_eq!(speed_color(0), BLUE);
        assert_eq!(speed_color(2 * 1024 * 1024), CYAN);
        assert_eq!(speed_color(50 * 1024 * 1024), VIOLET);
    }

    #[test]
    fn distance_to_a_segment() {
        assert_eq!(segment_distance((0.0, 3.0), (-1.0, 0.0), (1.0, 0.0)), 3.0);
        assert_eq!(segment_distance((4.0, 0.0), (-1.0, 0.0), (1.0, 0.0)), 3.0);
    }
}
