//! 剪贴板历史（`clipboard.jsonl`，一行一条，新的在前）。壳只负责读系统剪贴板、把复制的文字或图片交进来，
//! 存多少、去重、删、图片文件的去留都在这里，各平台共用。文字原样存（换行、缩进、制表符都在），所以用 jsonl 不用 TSV；
//! 图片另存在 `clipboard-images/`，按内容取名，同一张只存一份，条目删了文件跟着删。

mod entry;
mod history;

pub use entry::ClipboardEntry;
pub use history::{CLIPBOARD_FILE, CLIPBOARD_IMAGE_DIR, ClipboardHistory, DEFAULT_CLIPBOARD_LIMIT};
