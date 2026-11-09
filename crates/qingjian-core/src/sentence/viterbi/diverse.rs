//! 整句备选按「改的是哪里」排开：同一处的变体只留前几条在前面，其余挪到后面。
//!
//! 选词模型只重排候选窗前几条（均衡档 9 条），这几条全是句尾 啊 / 阿 / 吖 的变体时，句子中间错的字永远改不回来。

use super::Conversion;

/// 与已排在前面的备选改在同一处（改动的字有重叠）的，最多再留这么多条在前面。
const SAME_PLACE: usize = 1;

/// 第一条（最优路径）不动，其余按得分顺序挑：改动处与前面已挑的备选重叠超过 [`SAME_PLACE`] 次的挪到后面，
/// 挪走的之间仍按原顺序。字数与最优路径不同的（切分不同）当作改在别处。
pub(super) fn spread(paths: &mut Vec<Conversion>) {
    let Some(best) = paths.first() else {
        return;
    };
    let best: Vec<char> = best.text.chars().collect();
    let changed = |path: &Conversion| -> Option<Vec<usize>> {
        let chars: Vec<char> = path.text.chars().collect();
        (chars.len() == best.len())
            .then(|| (0..chars.len()).filter(|&i| chars[i] != best[i]).collect())
    };
    let mut rest = paths.split_off(1);
    let mut taken: Vec<Vec<usize>> = Vec::new();
    let mut later = Vec::new();
    for path in rest.drain(..) {
        let Some(places) = changed(&path) else {
            paths.push(path);
            continue;
        };
        let overlaps = taken
            .iter()
            .filter(|t| t.iter().any(|i| places.contains(i)))
            .count();
        if overlaps > SAME_PLACE {
            later.push(path);
        } else {
            taken.push(places);
            paths.push(path);
        }
    }
    paths.extend(later);
}
