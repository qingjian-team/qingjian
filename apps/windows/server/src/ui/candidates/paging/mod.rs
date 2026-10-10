//! Windows 候选窗的分页条，与候选窗一起定位、显示和收起，不激活输入焦点。

mod state;
mod window;

pub(super) use self::window::PagingWindow;
pub type PageClicks = Box<dyn Fn(isize) + Send>;
