//! UUID 的几种常用写法：表达式模式下敲 `vuuid`，全拼且问字键是 `u` 时直接敲 `uuid`。

use uuid::Uuid;

/// 表达式键 / 问字键之后（或整段）是这个词时出 UUID。
pub const UUID_KEYWORD: &str = "uuid";

/// 随机的 v4、按时间排序的 v7，再给 v4 的全大写与去掉连字符两种写法（与第一条同一个值）。每次调用都是新值。
pub fn uuid_forms() -> Vec<String> {
    let v4 = Uuid::new_v4();
    vec![
        v4.hyphenated().to_string(),
        Uuid::now_v7().hyphenated().to_string(),
        v4.hyphenated().to_string().to_ascii_uppercase(),
        v4.simple().to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::uuid_forms;

    #[test]
    fn forms_share_one_v4_and_add_a_v7() {
        let forms = uuid_forms();
        let [v4, v7, upper, simple] = forms.as_slice() else {
            panic!("{forms:?}");
        };
        assert_eq!(v4.len(), 36);
        assert_eq!(&v4[14..15], "4");
        assert_eq!(&v7[14..15], "7");
        assert_eq!(upper, &v4.to_ascii_uppercase());
        assert_eq!(simple, &v4.replace('-', ""));
        assert_ne!(uuid_forms()[0], *v4);
    }
}
