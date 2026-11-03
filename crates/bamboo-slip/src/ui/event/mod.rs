//! 交给交互层的输入事件，与窗口库无关：宿主把系统事件翻成它们（坐标已是像素，原点在根节点左上角）。

mod key;
mod modifiers;
mod text_input;

pub use key::{KeyCode, KeyPress};
pub use modifiers::Modifiers;
pub use text_input::TextInput;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    PointerMoved {
        x: f32,
        y: f32,
    },

    /// 指针离开窗口。
    PointerLeft,

    /// 主键按下 / 抬起；位置以最近一次 `PointerMoved` 为准。触屏的按下前宿主先补一个 `PointerMoved`。
    PointerDown,

    PointerUp,

    /// 右键按下；位置以最近一次 `PointerMoved` 为准。
    SecondaryDown,

    Key(KeyPress),

    /// 打出来的文字与输入法的组字，只交给拿着焦点、收文字的元素。
    Text(TextInput),

    /// 滚轮或触控板：滚动位置的增量（像素，向右、向下为正，即内容往左、往上走）。
    /// 宿主把行增量按行高换成像素；触控板的惯性阶段照样一个个送过来。
    Scroll {
        dx: f32,
        dy: f32,
    },
}
