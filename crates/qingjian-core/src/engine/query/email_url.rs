//! 中文模式邮箱 / 网址补全：在英文直输段上叠域名与后缀候选。

/// 候选窗右侧模式提示（与微信输入法措辞对齐）。
pub const MODE_HINT: &str = "退出网址、邮箱模式";

/// 常见邮箱域名（按国内常用优先）。
const EMAIL_DOMAINS: &[&str] = &[
    "qq.com",
    "163.com",
    "126.com",
    "yeah.net",
    "gmail.com",
    "263.net",
    "msn.com",
    "189.cn",
    "foxmail.com",
    "outlook.com",
    "sina.com",
    "icloud.com",
    "hotmail.com",
    "live.com",
    "me.com",
];

/// 网址后缀补全。
const URL_SUFFIXES: &[&str] = &[".com", ".cn", ".net", ".org", ".com.cn", ".edu.cn"];

/// 缓冲区是否进入邮箱 / 网址补全态（开关由调用方判断）。
pub fn is_email_url(scope: &str) -> bool {
    email_parts(scope).is_some() || url_stem(scope).is_some()
}

/// 生成补全候选文本：第一项始终是原文，后面跟匹配的域名 / 后缀。
pub fn completions(scope: &str) -> Vec<String> {
    let mut items = vec![scope.to_owned()];
    if let Some((local, domain_prefix)) = email_parts(scope) {
        for domain in EMAIL_DOMAINS {
            if !domain.starts_with(domain_prefix) {
                continue;
            }
            let full = format!("{local}@{domain}");
            if full != scope {
                items.push(full);
            }
        }
        return items;
    }
    if let Some(stem) = url_stem(scope) {
        for suffix in URL_SUFFIXES {
            // 已经以该后缀结尾就不必再补一条相同的
            if scope.eq_ignore_ascii_case(&format!("{stem}{suffix}")) {
                continue;
            }
            // 用户已经敲了部分后缀（`qingjian.c`）时按剩余前缀过滤
            let after = &scope[stem.len()..];
            if !suffix.starts_with(after) && !after.is_empty() {
                continue;
            }
            let full = format!("{stem}{suffix}");
            if full != scope {
                items.push(full);
            }
        }
    }
    items
}

/// `local@domain_prefix`：本地部分须为 ASCII 邮箱本地字符。
fn email_parts(scope: &str) -> Option<(&str, &str)> {
    let (local, domain) = scope.split_once('@')?;
    if local.is_empty()
        || !local
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
    {
        return None;
    }
    // 域名段只允许再敲域名字符；含第二个 `@` 或空格就不算
    if domain.contains('@')
        || !domain
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
    {
        return None;
    }
    Some((local, domain))
}

/// 可补 TLD 的词干：`www.` / `http(s)://` 前缀，或 host 形态（已含 `.` 的 ASCII）。
fn url_stem(scope: &str) -> Option<&str> {
    let lower = scope.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("www.") {
        return url_stem_from_prefix(scope);
    }
    // host 形态：字母数字开头，中间可有 `.-`，且至少有一个 `.`
    if !scope.contains('.') {
        return None;
    }
    if !scope
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '/' | ':' | '_' | '~' | '%' | '?' | '=' | '&'))
    {
        return None;
    }
    // 取最后一个尚未完成的「标签」之前的部分作为 stem：`qingjian.com` → stem `qingjian`
    // 若已是完整 TLD（`.com`），stem 仍是 host 名，补全其它后缀
    let host = scope.split(['/', '?', '#']).next().unwrap_or(scope);
    let (name, _) = host.rsplit_once('.')?;
    if name.is_empty() {
        return None;
    }
    // stem 保留到最后一个点之前（含路径前的 host 名）
    Some(name)
}

fn url_stem_from_prefix(scope: &str) -> Option<&str> {
    // 前缀本身就触发模式：`www.` → stem `www`，补 `www.com` 等意义不大；
    // 更合理：stem 为整段去掉已敲的不完整后缀，或整段作为 base 再挂后缀。
    // 用户敲 `www.` 时补 `www.com` / `www.cn`；敲 `www.qingjian` 时 stem=`www.qingjian`
    let lower = scope.to_ascii_lowercase();
    let prefix_len = if lower.starts_with("https://") {
        "https://".len()
    } else if lower.starts_with("http://") {
        "http://".len()
    } else if lower.starts_with("www.") {
        "www.".len()
    } else {
        return None;
    };
    let rest = &scope[prefix_len..];
    if rest.is_empty() {
        // `www.` / `http://`：以去掉末尾分隔符的前缀为 stem，补 `.com` → `www.com`；对 http 用 host 位
        let base = scope.trim_end_matches([':', '/']);
        return Some(base.trim_end_matches('.'));
    }
    // 有 host 内容：若已有点，stem 取最后一个点前；否则整段可补 `.com`
    if let Some((name, _)) = rest.rsplit_once('.') {
        if name.is_empty() {
            // `www.` 后立刻又一个点——少见，用整段去末点
            return Some(scope.trim_end_matches('.'));
        }
        let abs = prefix_len + name.len();
        return Some(&scope[..abs]);
    }
    Some(scope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_at_lists_domains() {
        let items = completions("qingjian@");
        assert_eq!(items[0], "qingjian@");
        assert!(items.iter().any(|s| s == "qingjian@yeah.net"));
        assert!(items.iter().any(|s| s == "qingjian@qq.com"));
    }

    #[test]
    fn email_prefix_filters_domains() {
        let items = completions("qingjian@qq");
        assert!(items.iter().all(|s| s == "qingjian@qq" || s.starts_with("qingjian@qq")));
        assert!(items.iter().any(|s| s == "qingjian@qq.com"));
        assert!(!items.iter().any(|s| s == "qingjian@gmail.com"));
    }

    #[test]
    fn www_dot_enters_url_mode() {
        assert!(is_email_url("www."));
        let items = completions("www.");
        assert_eq!(items[0], "www.");
        assert!(items.iter().any(|s| s == "www.com" || s == "www.cn"));
    }
}
