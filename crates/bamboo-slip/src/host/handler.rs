//! winit 的事件循环回调：开窗、翻译事件、按需重画。

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::StartCause;
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::{Theme, Window, WindowId};

use super::blit::blit;
use super::clipboard::clipboard_text;
use super::input::{key_code, modifiers, scroll};
use super::{HostError, WindowOptions};
use crate::app::{App, Runner};
use crate::ui::{Event, KeyCode, KeyPress, Modifiers, TextInput};

pub(super) struct Handler<A: App> {
    runner: Runner<A>,

    options: WindowOptions,

    /// 最近一次的修饰键，按键事件本身不带。
    modifiers: Modifiers,

    window: Option<Host>,

    /// 输入法开着（焦点在输入框里）时它的位置，与上次告诉系统的比，变了才再告诉。
    ime: Option<(f32, f32, f32, f32)>,

    /// 中途出的错，事件循环退出后交给调用方。
    pub(super) error: Option<HostError>,
}

/// 窗口和它的贴图面；softbuffer 的面要借着窗口，两个都用 `Arc` 持有。
struct Host {
    window: Arc<Window>,

    surface: Surface<Arc<Window>, Arc<Window>>,
}

impl<A: App> Handler<A> {
    pub(super) fn new(runner: Runner<A>, options: WindowOptions) -> Self {
        Self {
            runner,
            options,
            modifiers: Modifiers::default(),
            window: None,
            ime: None,
            error: None,
        }
    }

    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<(), HostError> {
        let (width, height) = self.options.size;
        let (min_width, min_height) = self.options.min_size;
        let attributes = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_inner_size(LogicalSize::new(width, height))
            .with_min_inner_size(LogicalSize::new(min_width, min_height));
        #[cfg(target_os = "macos")]
        let attributes = {
            use winit::platform::macos::WindowAttributesExtMacOS;
            attributes
                .with_titlebar_transparent(true)
                .with_title_hidden(true)
                .with_fullsize_content_view(true)
        };
        let window = Arc::new(event_loop.create_window(attributes)?);
        let context = Context::new(window.clone())?;
        let surface = Surface::new(&context, window.clone())?;
        self.runner
            .app_mut()
            .set_system_dark(window.theme() == Some(Theme::Dark));
        self.window = Some(Host { window, surface });
        Ok(())
    }

    /// 画下一帧，返回是否还有动画要接着画。
    fn redraw(&mut self) -> Result<bool, HostError> {
        let Some(host) = self.window.as_mut() else {
            return Ok(false);
        };
        let size = host.window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(false);
        };
        let scale = host.window.scale_factor() as f32;
        let start = Instant::now();
        let Some(painted) = self
            .runner
            .frame(scale, (width.get(), height.get()), start)?
        else {
            return Ok(false);
        };
        let painted_at = Instant::now();
        host.surface.resize(width, height)?;
        let mut buffer = host.surface.buffer_mut()?;
        blit(
            &painted.pixmap,
            (painted.content_x, painted.content_y),
            &mut buffer,
            width.get(),
            height.get(),
        );
        buffer.present()?;
        tracing::debug!(
            width = width.get(),
            height = height.get(),
            paint_ms = (painted_at - start).as_secs_f64() * 1000.0,
            blit_ms = painted_at.elapsed().as_secs_f64() * 1000.0,
            "重画"
        );
        Ok(painted.next_frame.is_some())
    }

    /// 焦点进出输入框时开关输入法，输入法的候选窗跟着输入框走；秘密输入（API 密钥这类）整段关掉，
    /// 键按原始键走，输入法组字与上报都不发生。
    fn sync_ime(&mut self, window: &Window) {
        let target = self
            .runner
            .interaction()
            .text_target()
            .filter(|_| self.runner.interaction().ime_allowed());
        if target == self.ime {
            return;
        }
        if target.is_some() != self.ime.is_some() {
            window.set_ime_allowed(target.is_some());
        }
        if let Some((x, y, width, height)) = target {
            window.set_ime_cursor_area(
                PhysicalPosition::new(x as f64, y as f64),
                PhysicalSize::new(width as f64, height as f64),
            );
        }
        self.ime = target;
    }

    /// 交给交互层；之后按要不要退出、重画收尾。
    fn dispatch(&mut self, event_loop: &ActiveEventLoop, window: &Window, input: &Event) {
        let redraw = self.runner.handle(input);
        if self.runner.quit_requested() {
            event_loop.exit();
        } else if redraw {
            window.request_redraw();
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: HostError) {
        tracing::error!(%error, "窗口宿主出错，退出");
        self.error = Some(error);
        event_loop.exit();
    }
}

impl<A: App> ApplicationHandler for Handler<A> {
    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause
            && let Some(host) = &self.window
        {
            host.window.request_redraw();
        }
    }

    /// 输入框有焦点时等到光标下一次翻转再醒，别的时候干等事件。
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(match self.runner.next_blink(Instant::now()) {
            Some(at) => ControlFlow::WaitUntil(at),
            None => ControlFlow::Wait,
        });
    }

    fn user_event(&mut self, _: &ActiveEventLoop, (): ()) {
        if self.runner.wake()
            && let Some(host) = &self.window
        {
            host.window.request_redraw();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.open(event_loop)
        {
            self.fail(event_loop, error);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref().map(|host| host.window.clone()) else {
            return;
        };
        let input = match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.runner.invalidate();
                window.request_redraw();
                return;
            }
            WindowEvent::Focused(focused) => {
                self.runner.set_active(focused);
                window.request_redraw();
                return;
            }
            WindowEvent::ThemeChanged(theme) => {
                self.runner.app_mut().set_system_dark(theme == Theme::Dark);
                window.request_redraw();
                return;
            }
            WindowEvent::RedrawRequested => {
                match self.redraw() {
                    // 还有动画在播：按显示器的刷新节奏接着要帧
                    Ok(true) => window.request_redraw(),
                    Ok(false) => {}
                    Err(error) => self.fail(event_loop, error),
                }
                self.sync_ime(&window);
                return;
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) => Event::Text(TextInput::Preedit(text)),
            WindowEvent::Ime(Ime::Commit(text)) => Event::Text(TextInput::Insert(text)),
            WindowEvent::ModifiersChanged(state) => {
                self.modifiers = modifiers(state.state());
                return;
            }
            WindowEvent::CursorMoved { position, .. } => Event::PointerMoved {
                x: position.x as f32,
                y: position.y as f32,
            },
            WindowEvent::CursorLeft { .. } => Event::PointerLeft,
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => Event::PointerDown,
                ElementState::Released => Event::PointerUp,
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = scroll(delta, window.scale_factor() as f32);
                Event::Scroll { dx, dy }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let key = key_code(&event.key_without_modifiers());
                let typing = self.runner.interaction().text_target().is_some();
                // 粘贴由宿主读剪贴板，当作打了这些字
                if typing
                    && self.modifiers.command
                    && key == Some(KeyCode::Character("v".to_owned()))
                {
                    if let Some(text) = clipboard_text() {
                        self.dispatch(event_loop, &window, &Event::Text(TextInput::Insert(text)));
                    }
                    return;
                }
                if let Some(key) = key {
                    self.dispatch(
                        event_loop,
                        &window,
                        &Event::Key(KeyPress::new(key, self.modifiers)),
                    );
                }
                // 带 ⌘ / Ctrl 的是快捷键，不算打字；回车、退格这类控制字符也不算
                let text = event
                    .text
                    .filter(|_| typing && !self.modifiers.command)
                    .map(|text| text.chars().filter(|c| !c.is_control()).collect::<String>())
                    .filter(|text| !text.is_empty());
                match text {
                    Some(text) => Event::Text(TextInput::Insert(text)),
                    None => return,
                }
            }
            _ => return,
        };
        self.dispatch(event_loop, &window, &input);
    }
}
