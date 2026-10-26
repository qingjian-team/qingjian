//! 宿主的错误：开不了窗口、事件循环出错、画不出来。

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("event loop failed: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),

    #[error("window creation failed: {0}")]
    Window(#[from] winit::error::OsError),

    #[error("surface failed: {0}")]
    Surface(#[from] softbuffer::SoftBufferError),

    #[error(transparent)]
    Render(#[from] crate::error::RenderError),
}
