//! 样例应用的状态与消息：照青简设置的「候选窗口」页（选项是真实的，只是不落盘）。`update` 是唯一改状态的地方。

use bamboo_slip::{Edit, TextEdit};

pub const PAGES: [&str; 11] = [
    "通用",
    "候选窗口",
    "快捷键",
    "自定义短语",
    "模糊音",
    "词库",
    "辅码",
    "云服务",
    "统计",
    "高级",
    "关于",
];

pub const APPEARANCES: [&str; 3] = ["跟随系统", "浅色", "深色"];

pub const LAYOUTS: [&str; 2] = ["竖排", "横排"];

pub const PINYIN: [&str; 3] = ["行内 + 窗口", "只在行内", "只在窗口"];

pub const ENGINES: [&str; 2] = ["青简渲染器", "系统绘制"];

/// 字号可调的范围（点）。
const SIZES: std::ops::RangeInclusive<u32> = 9..=40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Appearance,
    Layout,
    Pinyin,
    Engine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Candidate,
    Gloss,
}

#[derive(Debug, Clone)]
pub enum Message {
    Page(usize),
    Choose(Choice, usize),
    Expand,
    Step(Size, i32),
    ResetSizes,
    Animations,
    OpenFonts,
    Search(Edit),
    Quit,
}

pub struct Demo {
    pub page: usize,

    pub appearance: usize,

    pub layout: usize,

    /// 横排时 ↑ / ↓ 展开成多行。
    pub expand: bool,

    pub pinyin: usize,

    pub engine: usize,

    pub candidate_size: u32,

    pub gloss_size: u32,

    pub animations: bool,

    /// 系统是深色（「跟随系统」时用它）。
    pub system_dark: bool,

    /// 侧栏搜索框里的字（样例只能打字，不搜）。
    pub search: TextEdit,
}

impl Demo {
    pub fn new(system_dark: bool) -> Self {
        Self {
            page: 1,
            appearance: 0,
            layout: 0,
            expand: false,
            pinyin: 0,
            engine: 0,
            candidate_size: 17,
            gloss_size: 13,
            animations: true,
            system_dark,
            search: TextEdit::new(String::new()),
        }
    }

    pub fn dark(&self) -> bool {
        match self.appearance {
            1 => false,
            2 => true,
            _ => self.system_dark,
        }
    }

    pub fn choice(&self, choice: Choice) -> usize {
        match choice {
            Choice::Appearance => self.appearance,
            Choice::Layout => self.layout,
            Choice::Pinyin => self.pinyin,
            Choice::Engine => self.engine,
        }
    }

    pub fn size(&self, size: Size) -> u32 {
        match size {
            Size::Candidate => self.candidate_size,
            Size::Gloss => self.gloss_size,
        }
    }

    pub fn size_limits(&self, size: Size) -> (bool, bool) {
        let value = self.size(size);
        (value > *SIZES.start(), value < *SIZES.end())
    }

    /// 返回 `false` 表示要退出。
    pub fn update(&mut self, message: Message) -> bool {
        match message {
            Message::Page(page) => self.page = page,
            Message::Choose(choice, index) => {
                let slot = match choice {
                    Choice::Appearance => &mut self.appearance,
                    Choice::Layout => &mut self.layout,
                    Choice::Pinyin => &mut self.pinyin,
                    Choice::Engine => &mut self.engine,
                };
                *slot = index;
            }
            Message::Expand => self.expand = !self.expand,
            Message::Search(edit) => {
                self.search.apply(&edit);
            }
            Message::Step(size, delta) => {
                let slot = match size {
                    Size::Candidate => &mut self.candidate_size,
                    Size::Gloss => &mut self.gloss_size,
                };
                *slot = slot
                    .saturating_add_signed(delta)
                    .clamp(*SIZES.start(), *SIZES.end());
            }
            Message::ResetSizes => {
                self.candidate_size = 17;
                self.gloss_size = 13;
            }
            Message::Animations => self.animations = !self.animations,
            Message::OpenFonts => {}
            Message::Quit => return false,
        }
        true
    }
}
