/// 用户要求删掉一个候选（[`super::Engine::forget`]）之后，实际清掉了什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Forgotten {
    /// 它是用户词（云端词、自动造词、导入前学的词），已从用户词里删掉，本地不再出。
    pub user_word: bool,

    /// 清掉了对它的学习：选择次数、按输入串记的选择、个人 n-gram 里与它有关的转移。
    pub learning: bool,
}

impl Forgotten {
    /// 什么都没清（词库里的词，也没学过）。
    pub fn is_nothing(&self) -> bool {
        !self.user_word && !self.learning
    }
}
