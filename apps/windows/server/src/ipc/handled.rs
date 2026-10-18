//! 连接循环里一条消息交给工人线程后的结果。

use qingjian_platform::protocol::ServerMessage;

/// `pump` 里一条消息的分派结果。
pub enum Handled {
    /// 分派完了；`None` 是不回话的通知类消息。装箱是因为回话比 `Stop` 大得多。
    Reply(Option<Box<ServerMessage>>),

    /// 分派方放弃了（工人线程没了）：连接到此为止。
    Stop,
}
