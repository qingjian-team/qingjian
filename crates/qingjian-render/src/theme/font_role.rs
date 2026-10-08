//! 文字用途：同一帧的候选词与译词可选不同的字族。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FontRole {
    #[default]
    Ui,

    Candidate,

    Annotation,
}
