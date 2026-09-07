use super::Transition;

/// 刚上屏的那个候选记了哪些学习：用户紧接着把它整个退格删掉、再打同一段拼音换选别的词时，把这些记录退回去。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastCommit {
    /// 上屏的文本。
    pub text: String,

    /// 文本的字符数：退格这么多次就算整个删掉了。
    pub chars: usize,

    /// 消耗掉的那段拼音（按输入串记选择用的键）。
    pub input: String,

    /// 记过一次选择次数（`Learner::record`）与输入串选择（`record_choice`）的词；整句上屏没有。
    pub chosen: Option<String>,

    /// 记过的词转移（含上文与份数）。
    pub transitions: Vec<Transition>,

    /// 记进个人敲错表的 (敲的, 要的) 音节对。
    pub typos: Vec<(String, String)>,

    /// 上屏之后已经退格了几个字符。
    pub erased: usize,

    /// 输入日志里这次上屏的序号，撤销时指回去。
    pub log_id: u64,
}

impl LastCommit {
    /// 已经整个删掉了。
    pub fn is_erased(&self) -> bool {
        self.chars > 0 && self.erased == self.chars
    }

    /// 再打的这段拼音是不是同一段：完全相同，或是它的前缀（重打后只选了前面一个短词）。
    pub fn same_input(&self, input: &str) -> bool {
        !input.is_empty() && self.input.starts_with(input)
    }
}
