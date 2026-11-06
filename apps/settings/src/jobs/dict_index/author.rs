//! 一本词库的作者：名字，加一个可选的主页。主页地址来自网上取的清单，点名字会交给系统打开，所以只认像样的 https 域名。

use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Author {
    pub name: String,

    #[serde(default)]
    pub url: Option<String>,
}

impl Author {
    /// 能点开的主页；地址不安全（见 [`is_safe_link`]）当没有。
    pub fn link(&self) -> Option<&str> {
        self.url.as_deref().filter(|url| is_safe_link(url))
    }
}

/// 交给系统打开前的检查，与词库仓库 `tools/check.py` 收录时的规则一致：
/// 只要 https；不带用户名（`user@host` 能把真实域名藏在后面）；要是域名，不收 IP 与 `xn--` 开头的国际化域名（仿冒字形）。
pub fn is_safe_link(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return false;
    }
    let host = authority
        .rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map_or(authority, |(host, _)| host)
        .to_ascii_lowercase();
    let labels: Vec<&str> = host.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && !label.starts_with("xn--")
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && !labels
            .last()
            .is_some_and(|tld| tld.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::is_safe_link;

    #[test]
    fn only_plain_https_domains_open() {
        assert!(is_safe_link("https://qingjian.app"));
        assert!(is_safe_link("https://github.com/someone?tab=repositories"));
        assert!(is_safe_link("https://example.com:8443/me"));
        for bad in [
            "http://example.com",
            "file:///etc/passwd",
            "https://user@example.com",
            "https://evil.com@example.com",
            "https://192.168.0.1",
            "https://[::1]/",
            "https://xn--80ak6aa92e.com",
            "https://localhost",
            "javascript:alert(1)",
            "https://exa mple.com",
        ] {
            assert!(!is_safe_link(bad), "{bad}");
        }
    }
}
