//! `[[custom_phrases]]` 逐条读：写错的一条只丢它自己，其余短语与整份配置照常生效。
//! 读配置与偏好设置保存走同一套判定，保存时写错的条目原样留在文件里。

use qingjian_core::CustomPhrase;
use qingjian_core::custom_phrase::validate_phrases;
use serde::Deserialize;

/// 按顺序判定每一条：读不成短语、输入码或权重不合法、与前面的条目同码同文本的，给出原因。
pub(super) fn check(
    items: impl IntoIterator<Item = Result<CustomPhrase, String>>,
) -> Vec<Result<CustomPhrase, String>> {
    let mut accepted: Vec<CustomPhrase> = Vec::new();
    items
        .into_iter()
        .map(|item| {
            let phrase = item?;
            accepted.push(phrase);
            match validate_phrases(&accepted) {
                Ok(()) => Ok(accepted[accepted.len() - 1].clone()),
                Err(reason) => {
                    let phrase = accepted.pop().expect("刚放进去");
                    Err(format!("输入码 {}：{reason}", phrase.code))
                }
            }
        })
        .collect()
}

/// 读配置用：写错的条目记一条警告后跳过（第几条从 1 数，与文件里的顺序一致）。
pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<CustomPhrase>, D::Error> {
    let items = Vec::<toml::Value>::deserialize(deserializer)?;
    let checked = check(items.into_iter().map(|item| {
        item.try_into::<CustomPhrase>()
            .map_err(|e| e.message().to_owned())
    }));
    Ok(checked
        .into_iter()
        .enumerate()
        .filter_map(|(index, item)| {
            item.map_err(|reason| {
                tracing::warn!(entry = index + 1, %reason, "自定义短语有一条写错，已跳过，其余设置照常生效");
            })
            .ok()
        })
        .collect())
}

/// 0.3 之前的条目写的是固定位置：`position` 那一行原地换成 `weight`（值随后照常写入），注释跟着留下；
/// 已经有 `weight` 的只去掉 `position`。
pub(super) fn position_to_weight(table: &toml_edit::Table) -> toml_edit::Table {
    let mut renamed = toml_edit::Table::new();
    *renamed.decor_mut() = table.decor().clone();
    renamed.set_position(table.position());
    let has_weight = table.contains_key("weight");
    for (key, item) in table.iter() {
        let Some(formatted) = table.key(key) else {
            continue;
        };
        if key != "position" {
            renamed.insert_formatted(formatted, item.clone());
        } else if !has_weight {
            let weight =
                toml_edit::Key::new("weight").with_leaf_decor(formatted.leaf_decor().clone());
            renamed.insert_formatted(&weight, item.clone());
        }
    }
    renamed
}
