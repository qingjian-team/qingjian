/// 一个设置项在某个平台上的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// 输入法读这一项，设置程序显示它。
    Supported,
    /// 这个平台用不上（系统自己管，或没有对应的东西），不显示。
    NotApplicable,
    /// 输入法还没接上，接上之前不显示；逐条列在 `docs/plan/todo.md`。
    Planned,
}
