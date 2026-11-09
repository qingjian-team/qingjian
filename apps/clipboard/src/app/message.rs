//! [`Message`]：面板上发生的事。

use bamboo_slip::Edit;
use qingjian_clipboard::ClipFilter;

#[derive(Debug, Clone)]
pub enum Message {
    Search(Edit),

    /// 上下键移动选中项；`true` 往后。
    Move(bool),

    /// 粘贴选中项。
    PasteSelected,

    Paste(usize),

    /// 只放上剪贴板，不粘贴。
    Copy(usize),

    Filter(ClipFilter),

    /// 在这个位置（像素）给这一条开右键菜单。
    OpenMenu(usize, (f32, f32)),

    MenuPick(usize),

    CloseMenu,

    EditChange(Edit),

    EditSave,

    EditCancel,

    OpenDevices,

    CloseDevices,

    NameChange(Edit),

    NameSave,

    PairShowCode,

    /// 开始输入配对码，地址先填上（可能是空的）。
    PairEnter(String),

    PairAddress(Edit),

    PairCode(Edit),

    PairSubmit,

    PairCancel,

    Unpair(String),

    Close,
}
