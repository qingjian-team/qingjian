/// 选词模型给一条候选的分，拆开看：评测时看重排为什么挑错（[`super::super::Engine::last_choice_scores`]）。
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceScore {
    pub text: String,

    /// 在候选表里原来排第几（从 0 数）。
    pub rank: usize,

    /// 模型分（这页内 log-softmax 后）。
    pub model: f64,

    /// 语言模型分（短语三元或二元，已乘权重）。
    pub language_model: f64,

    /// 位置先验（已乘权重）。
    pub prior: f64,
}
