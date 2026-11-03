//! 一个元素怎么响应：点击产出的消息、按键与文字输入的处理、失去焦点、能不能拿焦点、是否禁用。

use taffy::NodeId;

use super::ScrollbarTrack;
use super::event::{KeyPress, TextInput};

/// 按键处理：处理了就返回消息，没处理返回 `None` 交给上级。
type KeyHandler<M> = Box<dyn Fn(&KeyPress) -> Option<M>>;

type TextHandler<M> = Box<dyn Fn(&TextInput) -> Option<M>>;

/// 拖动：指针在元素里的横向位置（0 为左边、1 为右边，出界夹到两头）。
type DragHandler<M> = Box<dyn Fn(f32) -> Option<M>>;

/// 右键：指针在根坐标里的位置（像素）。
type SecondaryHandler<M> = Box<dyn Fn((f32, f32)) -> M>;

/// 什么都不设也有用：元素会记悬停状态。
pub struct Interact<M> {
    pub(super) click: Option<M>,

    pub(super) secondary: Option<SecondaryHandler<M>>,

    pub(super) key: Option<KeyHandler<M>>,

    pub(super) text: Option<TextHandler<M>>,

    pub(super) drag: Option<DragHandler<M>>,

    /// 焦点离开它时的消息。
    pub(super) blur: Option<M>,

    pub(super) focusable: bool,

    /// 没有元素拿着焦点时自己拿（面板的搜索框）。
    pub(super) autofocus: bool,

    pub(super) disabled: bool,

    /// 秘密输入（API 密钥这类）：聚焦时宿主关掉输入法，键按原始键走，不让输入法组字并上报。
    pub(super) secret: bool,

    /// 滚动区：被滚动的内容节点（元素自己是视口）。
    pub(super) scroll_content: Option<NodeId>,

    /// 滚动条的轨道：按下与拖动由交互层直接改滚动位置。
    pub(super) scrollbar: Option<ScrollbarTrack>,
}

impl<M> Default for Interact<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Interact<M> {
    pub fn new() -> Self {
        Self {
            click: None,
            secondary: None,
            key: None,
            text: None,
            drag: None,
            blur: None,
            focusable: false,
            autofocus: false,
            disabled: false,
            secret: false,
            scroll_content: None,
            scrollbar: None,
        }
    }

    /// 点击（按下抬起都在它上面），以及有焦点时按空格 / 回车。
    pub fn on_click(self, message: M) -> Self {
        Self {
            click: Some(message),
            ..self
        }
    }

    /// 右键按下（上下文菜单这类）；交给指针下面最里层接它的元素。
    pub fn on_secondary(self, handler: impl Fn((f32, f32)) -> M + 'static) -> Self {
        Self {
            secondary: Some(Box::new(handler)),
            ..self
        }
    }

    pub fn on_key(self, handler: impl Fn(&KeyPress) -> Option<M> + 'static) -> Self {
        Self {
            key: Some(Box::new(handler)),
            ..self
        }
    }

    /// 收文字（输入框）：拿着焦点时打的字、输入法的组字交给它。收文字的元素都能拿焦点；
    /// 宿主据此打开输入法，并把输入法的候选窗放在它下面。
    pub fn on_text(self, handler: impl Fn(&TextInput) -> Option<M> + 'static) -> Self {
        Self {
            text: Some(Box::new(handler)),
            focusable: true,
            ..self
        }
    }

    /// 按下与按住拖动（滑块）：按下时捕获指针，之后每次移动都交给它，移出元素也算，抬起为止。
    pub fn on_drag(self, handler: impl Fn(f32) -> Option<M> + 'static) -> Self {
        Self {
            drag: Some(Box::new(handler)),
            ..self
        }
    }

    /// 焦点离开它（点了别处、Tab 走了）时的消息；元素从树里消失时不发。
    pub fn on_blur(self, message: M) -> Self {
        Self {
            blur: Some(message),
            ..self
        }
    }

    /// 能用 Tab 走到、点击时拿焦点。
    pub fn focusable(self) -> Self {
        Self {
            focusable: true,
            ..self
        }
    }

    /// 没有元素拿着焦点时自己拿，点了别处也回到它身上（一打开就能打字的搜索框）。
    pub fn autofocus(self, autofocus: bool) -> Self {
        Self { autofocus, ..self }
    }

    /// 禁用：不产出消息、不拿焦点，但仍挡住底下的元素。
    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// 秘密输入：聚焦时宿主关掉输入法（见 [`super::interaction::Interaction::ime_allowed`]），
    /// 打的键以原始键形式交给字段，贴秘密内容请用粘贴。跟着 [`Self::on_text`] 一起设。
    pub fn secret(self, secret: bool) -> Self {
        Self { secret, ..self }
    }

    /// 元素是滚动区的视口，`content` 是里面被滚动的内容：滚轮在它上面时滚它，滚动位置按元素的 key 记。
    pub fn scrolls(self, content: NodeId) -> Self {
        Self {
            scroll_content: Some(content),
            ..self
        }
    }

    /// 元素是滚动条的轨道，见 [`ScrollbarTrack`]。
    pub(crate) fn scrollbar(self, track: ScrollbarTrack) -> Self {
        Self {
            scrollbar: Some(track),
            ..self
        }
    }

    /// 能接点击：有点击消息且没禁用。
    pub(super) fn clickable(&self) -> bool {
        self.click.is_some() && !self.disabled
    }

    pub(super) fn draggable(&self) -> bool {
        self.drag.is_some() && !self.disabled
    }

    pub(super) fn takes_focus(&self) -> bool {
        self.focusable && !self.disabled
    }

    pub(super) fn accepts_text(&self) -> bool {
        self.text.is_some() && !self.disabled
    }
}
