//! 一个管道名上的接受循环要用的东西（新管道与过渡期的旧管道各一份）。

use std::sync::Arc;

use qingjian_platform::pipe::UserSid;

use super::limit::ConnectionLimit;

pub(super) struct Listener {
    pub(super) name: String,

    /// 后续实例的安全描述符（SDDL 文本，进了接受线程再转成描述符）。
    pub(super) sddl: String,

    /// Server 自己的用户：客户端得是同一个人。
    pub(super) me: UserSid,

    /// 新旧两个管道共用一份。
    pub(super) limit: Arc<ConnectionLimit>,
}
