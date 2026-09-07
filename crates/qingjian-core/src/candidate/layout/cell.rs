use super::super::Candidate;

/// 排布里的一格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell<'a> {
    /// 本地候选。
    Local(&'a Candidate),

    /// 云端词。
    Cloud(&'a Candidate),
}

impl<'a> Cell<'a> {
    pub fn candidate(self) -> &'a Candidate {
        match self {
            Cell::Local(c) | Cell::Cloud(c) => c,
        }
    }
}
