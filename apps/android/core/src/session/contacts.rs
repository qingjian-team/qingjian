//! 通讯录：设置里开了才由壳读系统通讯录交来；上屏名字后候选条接着给这个人的电话、邮箱（另一个开关）。

use qingjian_core::Contact;

use super::Session;
use crate::frame::{Frame, FrameCandidate};

impl Session {
    /// 换上通讯录（空列表即关掉），`details` 是上屏名字后给不给电话、邮箱。返回收进来的名字数。
    pub fn set_contacts(&mut self, contacts: &[Contact], details: bool) -> usize {
        let started = std::time::Instant::now();
        let count = self.engine.set_contacts(contacts);
        self.contact_details = details;
        self.after_contact = None;
        tracing::info!(
            contacts = contacts.len(),
            names = count,
            elapsed_ms = started.elapsed().as_millis(),
            "contacts loaded"
        );
        count
    }

    /// 刚上屏的是通讯录里的名字、组句也结束了：记下它，把它的电话、邮箱放进 `frame` 的候选（补全，选中原样上屏）。
    pub(super) fn offer_contact(&mut self, name: &str, frame: &mut Frame) {
        if !self.contact_details || !self.engine.composition().is_empty() {
            return;
        }
        let details = self.engine.contact_details(name).to_vec();
        if details.is_empty() {
            return;
        }
        for detail in details {
            frame.candidates.push(FrameCandidate::plain(&detail));
            self.completions.push((0, detail));
        }
        self.after_contact = Some(name.to_owned());
    }
}
