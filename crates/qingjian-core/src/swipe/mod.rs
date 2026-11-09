//! 滑行输入的解码：手指在 26 键上划过的轨迹加上键位几何，猜要打的字母串。
//! 平台壳只管判定滑行、记轨迹、画轨迹，把轨迹与各字母键的中心和大小交过来；解出来的字母串像一个个点出来的一样进组句，候选照常。
//! 设计见 `docs/design/voice-and-swipe.md`。

mod corner;
mod decoder;
mod key;
mod lexicon;
mod point;
mod shape;

#[cfg(test)]
mod fixtures;

pub use corner::CornerDecoder;
pub use decoder::SwipeDecoder;
pub use key::SwipeKey;
pub use lexicon::SwipeLexicon;
pub use point::SwipePoint;
pub use shape::ShapeDecoder;
