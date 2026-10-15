//! 搜索跳转后的醒目标记。

use std::time::Instant;

use bamboo_slip::Key;

use super::Settings;
use crate::search::SPOTLIGHT;

impl Settings {
    /// 醒目标出跳到的那一行，到时间叫醒窗口撤掉（撤掉时底色淡出）。
    pub(super) fn spotlight(&mut self, key: Option<Key>) {
        let Some(key) = key else {
            self.search.spotlight = None;
            return;
        };
        self.search.spotlight = Some((key, Instant::now() + SPOTLIGHT));
        if let Some(waker) = self.waker.clone() {
            std::thread::spawn(move || {
                std::thread::sleep(SPOTLIGHT);
                waker.wake();
            });
        }
    }
}
