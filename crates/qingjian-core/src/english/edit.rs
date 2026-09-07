//! 两个小写串是否只差一处编辑（替换、插入、删除、相邻换位）。

/// `a` 与 `b` 相等，或只差一处编辑：换一个字母、多 / 少一个字母、相邻两个字母对调。
/// 英文模式拼错一个字母时用它找词，串都很短，逐字节比就够了。
pub fn within_one_edit(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    match a.len().abs_diff(b.len()) {
        0 => same_length(a, b),
        1 => {
            let (short, long) = if a.len() < b.len() { (a, b) } else { (b, a) };
            one_insertion(short, long)
        }
        _ => false,
    }
}

/// 等长：最多一个位置不同，或恰好相邻两个位置对调。
fn same_length(a: &[u8], b: &[u8]) -> bool {
    let mut differences = a.iter().zip(b).enumerate().filter(|(_, (x, y))| x != y);
    let Some((first, _)) = differences.next() else {
        return true;
    };
    match differences.next() {
        None => true,
        Some((second, _)) => {
            second == first + 1
                && a[first] == b[second]
                && a[second] == b[first]
                && differences.next().is_none()
        }
    }
}

/// `long` 去掉一个字节后等于 `short`。
fn one_insertion(short: &[u8], long: &[u8]) -> bool {
    let mut i = 0;
    let mut skipped = false;
    for &byte in long {
        if i < short.len() && short[i] == byte {
            i += 1;
        } else if skipped {
            return false;
        } else {
            skipped = true;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_each_kind_of_edit() {
        assert!(within_one_edit("hello", "hello"));
        assert!(within_one_edit("hallo", "hello")); // 替换
        assert!(within_one_edit("helo", "hello")); // 少一个
        assert!(within_one_edit("helllo", "hello")); // 多一个
        assert!(within_one_edit("hlelo", "hello")); // 换位
        assert!(within_one_edit("ello", "hello")); // 开头少一个
        assert!(within_one_edit("hell", "hello")); // 结尾少一个
    }

    #[test]
    fn rejects_two_edits() {
        assert!(!within_one_edit("hxllx", "hello"));
        assert!(!within_one_edit("hlleo", "hello"));
        assert!(!within_one_edit("he", "hello"));
        assert!(!within_one_edit("hello", "olleh"));
        assert!(!within_one_edit("abcd", "acbe"));
    }
}
