//! 外观页的两个字号步进框：候选字号、译文字号。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeKind {
    Candidate,
    Annotation,
}
