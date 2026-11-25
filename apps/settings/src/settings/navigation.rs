//! 换页与前后翻：侧栏点一页、工具栏的前后箭头（按看过的页走，跟系统设置一样）。

use super::Settings;
use crate::library::Library;
use crate::message::FieldId;
use crate::page_id::PageId;
use crate::stats::Stats;

impl Settings {
    /// 换到 `page`：补存输入框、关掉这页的临时状态，进有些页时重新读一遍数据。
    pub(super) fn open_page(&mut self, page: PageId) {
        // 输入框在别的页上会直接消失、收不到失焦，这里补存
        for id in FieldId::ALL {
            self.commit(id);
        }
        self.page = page;
        self.recording = None;
        self.close_phrase();
        if page == PageId::Dictionaries {
            self.library = Library::scan();
        }
        if page == PageId::Input {
            self.library.rescan_schemes();
        }
        if page == PageId::Stats {
            self.stats = Some(Stats::load(&self.config.general));
        }
        if page == PageId::Vocabulary {
            self.open_vocabulary();
        }
    }
}
