//! 候选列表 UI 元素（`ITfCandidateListUIElement`）：全屏游戏这类自己画候选的应用从这里读候选，
//! 它们说自己画时 Server 的候选窗不画（全屏时那个弹窗本来就被挡住）。起止在 [`list`]，元素对象在 [`element`]。

mod element;
mod list;

use self::element::CandidateElement;
pub(crate) use self::list::CandidateList;
