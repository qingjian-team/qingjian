//! 主题继承：`"extends": "qingjian"` 以那个内置主题为底，本文件写的覆盖上去。在 JSON 层合并：
//! 对象逐键合并（`variables` 只写要改的几个颜色即可），数组与标量整个替换（`children` 写了就是整组换掉）。

use serde_json::{Map, Value};

use super::{ThemeError, jsonc};

/// 继承链最多几层，挡住互相继承。
const MAX_DEPTH: usize = 4;

/// 展开 `extends`，返回合并后的整份主题。`base` 按 id 给出内置主题的源文件。
/// 没写 `extends` 的一律以 `root`（青简绿）为底，内置主题与继承链上的每一层都一样，只写几个颜色也能用；
/// `root` 自己是最底层，不再往下找（不然自己继承自己）。
pub(super) fn resolve(
    json: &str,
    base: &dyn Fn(&str) -> Option<&'static str>,
    root: &str,
) -> Result<Value, ThemeError> {
    let value: Value = serde_json::from_str(&jsonc::strip(json))?;
    let own = value
        .pointer("/meta/id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    resolve_value(value, own.as_deref(), base, root, 0)
}

/// 主题文件顶层 `extends` 写的 id（只看这一层）；没写或读不出来为 `None`。
pub(super) fn base_id(json: &str) -> Option<String> {
    let value: Value = serde_json::from_str(&jsonc::strip(json)).ok()?;
    value.get("extends")?.as_str().map(str::to_owned)
}

/// `own` 是这一层主题自己的 id（顶层取 `meta.id`，往下是正在展开的内置主题）。
fn resolve_value(
    mut value: Value,
    own: Option<&str>,
    base: &dyn Fn(&str) -> Option<&'static str>,
    root: &str,
    depth: usize,
) -> Result<Value, ThemeError> {
    let written = value
        .as_object_mut()
        .and_then(|object| object.remove("extends"));
    let id = match written {
        Some(id) => id,
        None if own == Some(root) => return Ok(value),
        None => Value::String(root.to_owned()),
    };
    let Value::String(id) = id else {
        return Err(ThemeError::UnknownBase { id: id.to_string() });
    };
    if depth >= MAX_DEPTH {
        return Err(ThemeError::TooDeep { id });
    }
    let source = base(&id).ok_or_else(|| ThemeError::UnknownBase { id: id.clone() })?;
    let mut merged = resolve_value(
        serde_json::from_str(&jsonc::strip(source))?,
        Some(&id),
        base,
        root,
        depth + 1,
    )?;
    merge(&mut merged, value);
    Ok(merged)
}

/// 把 `over` 合并进 `base`：两边都是对象就逐键递归，否则整个替换。
fn merge(base: &mut Value, over: Value) {
    match (base, over) {
        (Value::Object(base), Value::Object(over)) => merge_objects(base, over),
        (base, over) => *base = over,
    }
}

fn merge_objects(base: &mut Map<String, Value>, over: Map<String, Value>) {
    for (key, value) in over {
        match base.get_mut(&key) {
            Some(existing) => merge(existing, value),
            None => {
                base.insert(key, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r##"{ "a": { "x": 1, "y": [1, 2] }, "b": "keep" }"##;

    fn base(id: &str) -> Option<&'static str> {
        match id {
            "base" => Some(BASE),
            "loop" => Some(r#"{ "extends": "loop" }"#),
            "middle" => Some(r#"{ "m": 1 }"#),
            _ => None,
        }
    }

    #[test]
    fn merges_objects_and_replaces_arrays() {
        let merged = resolve(
            r#"{ "extends": "base", "a": { "y": [3] }, "c": 1 }"#,
            &base,
            "base",
        )
        .unwrap();
        assert_eq!(
            merged,
            serde_json::json!({ "a": { "x": 1, "y": [3] }, "b": "keep", "c": 1 })
        );
    }

    #[test]
    fn rejects_unknown_and_cyclic_bases() {
        assert!(matches!(
            resolve(r#"{ "extends": "nope" }"#, &base, "base"),
            Err(ThemeError::UnknownBase { .. })
        ));
        assert!(matches!(
            resolve(r#"{ "extends": "loop" }"#, &base, "base"),
            Err(ThemeError::TooDeep { .. })
        ));
    }

    #[test]
    fn missing_extends_falls_back_to_root_at_every_level() {
        // 顶层没写
        let top = resolve(r#"{ "c": 1 }"#, &base, "base").unwrap();
        assert_eq!(top["b"], "keep");
        // 中间一层没写（内置主题忘了写）也接到最底层
        let middle = resolve(r#"{ "extends": "middle" }"#, &base, "base").unwrap();
        assert_eq!(
            (&middle["m"], &middle["b"]),
            (&serde_json::json!(1), &serde_json::json!("keep"))
        );
        // 最底层自己不再往下找
        let root = resolve(r#"{ "meta": { "id": "base" }, "z": 0 }"#, &base, "base").unwrap();
        assert!(root.get("b").is_none());
    }
}
