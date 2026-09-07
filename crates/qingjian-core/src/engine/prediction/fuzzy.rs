//! 云端词的拼音是否对得上用户敲的字母：允许简拼、少量错字 / 漏字 / 多字。
//!
//! 模型的价值之一就是纠错（`zhgdoima` → 这个东西吗），所以不能要求逐音节精确相符；
//! 但也不能模型说什么就是什么，否则一个无关的词会插进候选。这里算「用户的字母」与「该词全拼」之间的编辑距离，
//! 其中每个音节允许只取前缀（简拼不算错），超过容许的错误数就丢。

/// 用户字母 `typed` 匹配到全拼音节 `syllables` 所需的最少错误数（替换 / 多敲 / 漏敲各算 1）。
/// 每个音节可以只匹配它的一个非空前缀，省掉的后缀不算错。
pub fn mismatch_count(typed: &str, syllables: &[String]) -> usize {
    let typed: Vec<u8> = typed.bytes().filter(|b| *b != b'\'').collect();
    let n = typed.len();
    const INF: usize = usize::MAX / 2;
    // dp[i]：typed 的前 i 个字母匹配完前面所有音节的最少错误数
    let mut dp = vec![INF; n + 1];
    dp[0] = 0;
    for syllable in syllables {
        let s = syllable.as_bytes();
        let mut next = vec![INF; n + 1];
        // 音节的每个非空前缀都是一种可能的写法；前缀 p 与 typed[i..j] 的编辑距离经典 DP
        for start in 0..=n {
            if dp[start] == INF {
                continue;
            }
            // row[k][j-start]：前缀长度 k 与 typed[start..j] 的编辑距离
            let mut row: Vec<usize> = (0..=n - start).collect();
            for k in 1..=s.len() {
                let mut new_row = vec![0; n - start + 1];
                new_row[0] = k;
                for j in 1..=n - start {
                    let cost = usize::from(s[k - 1] != typed[start + j - 1]);
                    new_row[j] = (row[j - 1] + cost).min(row[j] + 1).min(new_row[j - 1] + 1);
                }
                row = new_row;
                // 到这里前缀长度 k 已完整，可作为该音节的写法收尾
                for j in 1..=n - start {
                    let total = dp[start] + row[j];
                    if total < next[start + j] {
                        next[start + j] = total;
                    }
                }
            }
        }
        dp = next;
    }
    dp[n]
}

/// 容许的错误数：不到 4 个字母不容错（`zt` 容 1 个错就什么词都对得上），4 个起容 1 个，之后每 6 个字母多容 1 个。
pub fn tolerance(letters: usize) -> usize {
    if letters < 4 {
        0
    } else {
        1 + (letters - 4) / 6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syl(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn exact_and_abbreviated_pinyin_cost_nothing() {
        assert_eq!(mismatch_count("zhangtao", &syl(&["zhang", "tao"])), 0);
        assert_eq!(mismatch_count("zt", &syl(&["zhang", "tao"])), 0);
        assert_eq!(
            mismatch_count("zh'g'do'ima", &syl(&["zhe", "ge", "dong", "xi", "ma"])),
            1
        );
        assert_eq!(mismatch_count("kaifa", &syl(&["kai", "fa"])), 0);
    }

    #[test]
    fn typos_are_counted_and_unrelated_words_rejected() {
        // 错一个字母
        assert_eq!(mismatch_count("kaufa", &syl(&["kai", "fa"])), 1);
        // 多敲一个
        assert_eq!(mismatch_count("kaifaa", &syl(&["kai", "fa"])), 1);
        // 完全无关
        assert!(mismatch_count("zhangtao", &syl(&["zhi", "dao"])) >= 3);
        assert!(mismatch_count("zt", &syl(&["kai", "fa"])) >= 2);
        // 音节数不对也会体现在错误数上
        assert!(mismatch_count("zhangtao", &syl(&["zhang", "tao", "tao"])) >= 1);
    }

    #[test]
    fn tolerance_grows_slowly_with_length() {
        assert_eq!(tolerance(2), 0);
        assert_eq!(tolerance(3), 0);
        assert_eq!(tolerance(5), 1);
        assert_eq!(tolerance(9), 1);
        assert_eq!(tolerance(10), 2);
    }
}
