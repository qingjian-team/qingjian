//! 词图的跨度：一段音节位置上查到的词（`SpanWord`）与按输入串缓存的查词结果（`SpanCache`）。

mod cache;
mod word;

pub use cache::{MAX_SPAN_CACHE_ENTRIES, SpanCache};
pub use word::SpanWord;
