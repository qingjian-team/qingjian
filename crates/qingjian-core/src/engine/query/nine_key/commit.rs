//! 九宫格上屏时消耗多少缓冲区。

use crate::candidate::Candidate;
use crate::engine::Engine;

impl Engine {
    /// 候选按音节长度消耗九宫格缓冲区（一个字母一个数字，读法与数字串等长）：跳过 `'`，不超过作用域；
    /// 末尾没打完的音节（候选音节比剩下的输入长）把输入吃完。
    pub(in crate::engine) fn nine_key_consumed(&self, candidate: &Candidate) -> usize {
        let scope = self.composition.scope().as_bytes();
        let mut consumed = 0;
        for syllable in &candidate.syllables {
            if consumed > 0 && scope.get(consumed) == Some(&b'\'') {
                consumed += 1;
            }
            if consumed >= scope.len() {
                break;
            }
            let span = scope[consumed..]
                .iter()
                .take(syllable.len())
                .take_while(|byte| **byte != b'\'')
                .count();
            consumed += span;
        }
        if scope.get(consumed) == Some(&b'\'') {
            consumed += 1;
        }
        consumed.min(scope.len())
    }
}
