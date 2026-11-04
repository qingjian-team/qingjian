//! `[[custom_phrases]]` 逐条读：写错的一条只丢它自己，其余短语与整份配置照常生效。
//! 读配置与偏好设置保存走同一套判定，保存时写错的条目原样留在文件里。

use qingjian_core::CustomPhrase;
use qingjian_core::custom_phrase::{DEFAULT_WEIGHT, WEIGHTS, validate_phrases};
use serde::Deserialize;

use crate::i18n::{tr, tr_args};

/// 按顺序判定每一条：读不成短语、输入码或权重不合法、与前面的条目同码同文本的，给出原因（界面语言写的）。
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
                    accepted.pop();
                    Err(reason)
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

/// 导出的文件：只有 `[[custom_phrases]]` 的一份 TOML，与配置文件里的写法相同（能存多行短语，TSV 存不了），
/// 用文本编辑器打开也能改；[`import_phrases`] 读回。
pub fn export_phrases(phrases: &[CustomPhrase]) -> String {
    let mut tables = toml_edit::ArrayOfTables::new();
    for phrase in phrases {
        let mut table = toml_edit::Table::new();
        table["code"] = toml_edit::value(phrase.code.as_str());
        table["text"] = toml_edit::value(phrase.text.as_str());
        table["weight"] = toml_edit::value(i64::from(phrase.weight));
        table["enabled"] = toml_edit::value(phrase.enabled);
        tables.push(table);
    }
    let mut document = toml_edit::DocumentMut::new();
    document["custom_phrases"] = toml_edit::Item::ArrayOfTables(tables);
    format!(
        "# 青简自定义短语，在设置程序「自定义短语」页导入；写法与配置文件里的 [[custom_phrases]] 相同\n\n{document}"
    )
}
/// 读导出的短语文件里的 `[[custom_phrases]]`：返回读得出的短语，与写错跳过的每一条在哪、为什么（按文件顺序，界面语言写的，
/// 用户照着改文件：码里带数字这类错不说原因就只知道「几条写错」，#857）。
/// 读不成 TOML 的按「文字、Tab、输入码、Tab、权重」一行一条读（Rime `custom_phrase.txt` 的写法，方便从别的输入法搬）。
/// 整份配置文件不收：设置与短语各自导入，免得用户以为导了短语、设置也跟着变了。
pub fn import_phrases(source: &str) -> Result<(Vec<CustomPhrase>, Vec<String>), String> {
    let Ok(mut table) = source.parse::<toml::Table>() else {
        return import_tsv(source);
    };
    let items = match table.remove("custom_phrases") {
        Some(toml::Value::Array(items)) => items,
        _ if table.is_empty() => return import_tsv(source),
        _ => return Err(tr("phrase_file.empty").to_owned()),
    };
    if !table.is_empty() {
        return Err(tr("phrase_file.is_config").to_owned());
    }
    let checked = check(items.into_iter().map(|item| {
        item.try_into::<CustomPhrase>()
            .map_err(|e| e.message().to_owned())
    }));
    Ok(split(checked.into_iter().enumerate().map(
        |(index, item)| {
            item.map_err(|reason| {
                tr_args(
                    "phrase_file.skipped_entry",
                    &[("entry", &(index + 1)), ("reason", &reason)],
                )
            })
        },
    )))
}

/// 读出来的短语与跳过的说明分开。
fn split(
    items: impl Iterator<Item = Result<CustomPhrase, String>>,
) -> (Vec<CustomPhrase>, Vec<String>) {
    let mut phrases = Vec::new();
    let mut skipped = Vec::new();
    for item in items {
        match item {
            Ok(phrase) => phrases.push(phrase),
            Err(reason) => skipped.push(reason),
        }
    }
    (phrases, skipped)
}

/// 「文字、Tab、输入码、Tab、权重」一行一条，权重可省；`#` 开头的行与 Rime 文件头（`---` 到 `...`）跳过。
/// 权重夹到 [`WEIGHTS`] 里（Rime 的权重常上千），没写用缺省；一行里换行写不进去，多行短语请用导出的 TOML。
fn import_tsv(source: &str) -> Result<(Vec<CustomPhrase>, Vec<String>), String> {
    let mut header = false;
    let mut items = Vec::new();
    // 第几行（从 1 数）与那行的文字，跳过时用来说是哪一条
    let mut places = Vec::new();
    for (number, line) in source.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line == "---" {
            header = true;
            continue;
        }
        if header {
            header = line != "...";
            continue;
        }
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (text, code, weight) = (fields.next(), fields.next(), fields.next());
        places.push((number + 1, text.unwrap_or_default().to_owned()));
        items.push(match (text, code) {
            (Some(text), Some(code)) => Ok(CustomPhrase {
                weight: weight
                    .and_then(|weight| weight.trim().parse::<u64>().ok())
                    .map_or(DEFAULT_WEIGHT, |weight| {
                        weight.clamp(u64::from(*WEIGHTS.start()), u64::from(*WEIGHTS.end())) as u32
                    }),
                ..CustomPhrase::new(code.trim(), text)
            }),
            _ => Err(tr("phrase_file.no_tab").to_owned()),
        });
    }
    if items.is_empty() {
        return Err(tr("phrase_file.unreadable").to_owned());
    }
    let checked = check(items);
    Ok(split(checked.into_iter().zip(places).map(
        |(item, (line, text))| {
            item.map_err(|reason| {
                tr_args(
                    "phrase_file.skipped_line",
                    &[
                        ("line", &line),
                        ("text", &shortened(&text)),
                        ("reason", &reason),
                    ],
                )
            })
        },
    )))
}

/// 跳过的说明里引用那行文字：长的只留开头几个字。
fn shortened(text: &str) -> String {
    const SHOWN: usize = 10;
    match text.char_indices().nth(SHOWN) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
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
