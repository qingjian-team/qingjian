//! 青简剪贴板的平台无关部分。这里不放窗口与界面。

mod capture;
mod edge;
mod error;
mod item;
mod kind;
mod settings;
mod store;

pub use capture::{
    Capture, ClipFormat, FORMAT_FILE_SIZE, FORMAT_FILES, FORMAT_HTML, FORMAT_PNG, FORMAT_RTF,
    FORMAT_THUMBNAIL, MAX_FILE_LIST_BYTES, MAX_FORMAT_BYTES, MAX_IMAGE_BYTES, MAX_TEXT_CHARS,
    format_size,
};
pub use edge::{Edge, WorkArea};
pub use error::ClipboardError;
pub use item::ClipItem;
pub use kind::{ClipFilter, ClipKind};
pub use settings::{ClipSettings, Hotkey};
pub use store::{ClipStore, ImageDecoder, StoreReplica};
