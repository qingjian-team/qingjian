use std::cmp::Ordering;

/// 版本号，按语义化版本的规则比大小：`0.1.4-beta.1 < 0.1.4-beta.2 < 0.1.4-rc.1 < 0.1.4`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],

    /// 预发布后缀按 `.` 拆开的各段；空 = 正式版。
    pre: Vec<String>,
}

impl Version {
    /// 解析 `主.次.补丁[-后缀]`；构建元数据（`+` 之后）丢掉。
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut numbers = core.split('.').map(|part| part.parse::<u64>().ok());
        let core = [numbers.next()??, numbers.next()??, numbers.next()??];
        if numbers.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => Vec::new(),
            // `1.2.3-`、`1.2.3-beta.` 这种预发布段有空段的不是合法版本号
            Some("") => return None,
            Some(pre) => {
                let parts: Vec<String> = pre.split('.').map(str::to_owned).collect();
                if parts.iter().any(|part| part.is_empty()) {
                    return None;
                }
                parts
            }
        };
        Some(Self { core, pre })
    }

    /// 本地开发包（`0.1.3-dev-1a2b3c4`）：不检查更新。
    pub fn is_dev(&self) -> bool {
        self.pre.first().is_some_and(|part| part.starts_with("dev"))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core
            .cmp(&other.core)
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => compare_pre(&self.pre, &other.pre),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 逐段比：都是数字按数值，数字小于字母段，其余按字典序；前面都相同时段少的小。
fn compare_pre(left: &[String], right: &[String]) -> Ordering {
    for (a, b) in left.iter().zip(right) {
        let ordering = match (a.parse::<u64>(), b.parse::<u64>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => a.cmp(b),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn orders_like_semver() {
        let ordered = [
            "0.1.3",
            "0.1.4-alpha.1",
            "0.1.4-beta.1",
            "0.1.4-beta.2",
            "0.1.4-beta.10",
            "0.1.4-rc.1",
            "0.1.4",
            "0.2.0",
            "0.10.0",
        ];
        for pair in ordered.windows(2) {
            assert!(
                version(pair[0]) < version(pair[1]),
                "{} < {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn recognizes_dev_builds() {
        assert!(version("0.1.3-dev").is_dev());
        assert!(version("0.1.3-dev-d893e1e+").is_dev());
        assert!(!version("0.1.4-beta.1").is_dev());
        assert!(!version("0.1.3").is_dev());
    }

    #[test]
    fn rejects_malformed_versions() {
        for text in [
            "",
            "0.1",
            "0.1.x",
            "0.1.2.3",
            "v0.1.3",
            "0.1.4-",
            "0.1.4-.",
            "0.1.4-beta.",
        ] {
            assert!(Version::parse(text).is_none(), "{text}");
        }
    }
}
