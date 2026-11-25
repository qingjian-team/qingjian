//! 把画好的位图贴到窗口：一般走 softbuffer；macOS 上带系统材质的窗口走自己的 CALayer（softbuffer 在 macOS 上丢掉 alpha，透不出材质）。

mod blit;
#[cfg(target_os = "macos")]
mod layer;

use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Context, Surface};
use winit::window::Window;

use super::HostError;
use crate::backdrop::Backdrop;
use crate::stage::Painted;
use blit::blit;

pub(super) enum Presenter {
    Soft(Surface<Arc<Window>, Arc<Window>>),

    #[cfg(target_os = "macos")]
    Layer(layer::Layer),
}

impl Presenter {
    /// macOS 上要了 [`Backdrop::Window`] 就在窗口后面垫毛玻璃、自己贴图；垫不上退回 softbuffer。
    pub(super) fn new(window: &Arc<Window>, backdrop: Option<Backdrop>) -> Result<Self, HostError> {
        #[cfg(target_os = "macos")]
        if backdrop == Some(Backdrop::Window) {
            match layer::Layer::new(window) {
                Some(layer) => return Ok(Self::Layer(layer)),
                None => {
                    tracing::warn!("no AppKit view for the backdrop, falling back to softbuffer")
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = backdrop;
        let context = Context::new(window.clone())?;
        Ok(Self::Soft(Surface::new(&context, window.clone())?))
    }

    /// 位图里内容区左上角对齐窗口左上角，按窗口当前的物理像素大小贴；窗口最小化（零大小）时不贴。
    pub(super) fn present(&mut self, window: &Window, painted: &Painted) -> Result<(), HostError> {
        let size = window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(());
        };
        let origin = (painted.content_x, painted.content_y);
        match self {
            Self::Soft(surface) => {
                surface.resize(width, height)?;
                let mut buffer = surface.buffer_mut()?;
                blit(
                    &painted.pixmap,
                    origin,
                    &mut buffer,
                    width.get(),
                    height.get(),
                );
                buffer.present()?;
            }
            #[cfg(target_os = "macos")]
            Self::Layer(layer) => {
                let (width, height) = (width.get(), height.get());
                let mut pixels = vec![0; (width * height) as usize];
                blit(&painted.pixmap, origin, &mut pixels, width, height);
                layer.present(&pixels, width, height, window.scale_factor());
            }
        }
        Ok(())
    }
}
