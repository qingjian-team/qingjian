//! 读取雾凇的声明式数据，使用 YAML 解析器而不执行配置或 Lua。

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use super::profile::Profile;
use crate::emoji::EmojiTable;
use qingjian_dictionary::{Dictionary, WordList, import};
use yaml_rust2::{Yaml, YamlLoader};

fn invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

fn read(root: &Path, name: &str) -> io::Result<String> {
    let path = root.join(name).canonicalize()?;
    if !path.starts_with(root) {
        return Err(invalid("data path escapes source directory"));
    }
    std::fs::read_to_string(path)
}

fn yaml(text: &str) -> io::Result<Yaml> {
    YamlLoader::load_from_str(text)
        .map_err(invalid)?
        .into_iter()
        .next()
        .ok_or_else(|| invalid("empty YAML"))
}

fn table(text: &str) -> HashMap<String, Vec<String>> {
    let mut table: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    for line in text
        .lines()
        .filter(|s| !s.trim().is_empty() && !s.starts_with('#'))
    {
        let mut parts = line.split('\t');
        if let (Some(word), Some(code)) = (parts.next(), parts.next()) {
            let words = table.entry(code.to_ascii_lowercase()).or_default();
            if !words.iter().any(|(w, _)| w == word) {
                let weight = parts
                    .next()
                    .and_then(|value| value.parse::<i64>().ok())
                    .unwrap_or(1);
                if weight > 0 {
                    words.push((word.to_owned(), weight));
                }
            }
        }
    }
    table
        .into_iter()
        .map(|(code, mut words)| {
            words.sort_by_key(|(_, weight)| std::cmp::Reverse(*weight));
            (code, words.into_iter().map(|(word, _)| word).collect())
        })
        .collect()
}

fn strings(value: &Yaml) -> Vec<String> {
    value
        .as_vec()
        .into_iter()
        .flatten()
        .filter_map(Yaml::as_str)
        .map(str::to_owned)
        .collect()
}

impl Profile {
    pub(super) fn load(root: &Path, cache: &Path) -> io::Result<Self> {
        let schema = yaml(&read(root, "rime_ice.schema.yaml")?)?;
        // 以源文件内容指纹区分缓存，改中文分表后自动重新导入；失败不替换已有词库。
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        read(root, "rime_ice.dict.yaml")?.hash(&mut hash);
        let mut tables: Vec<PathBuf> = std::fs::read_dir(root.join("cn_dicts"))?
            .collect::<io::Result<Vec<_>>>()?
            .into_iter()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        tables.sort();
        for path in tables {
            if !path.canonicalize()?.starts_with(root) {
                return Err(invalid("dictionary path escapes source directory"));
            }
            path.file_name().hash(&mut hash);
            std::fs::read(path)?.hash(&mut hash);
        }
        let cache = cache.join(format!("{:016x}", hash.finish()));
        let converted = cache.join("rime_ice.qj");
        let dictionary = if converted.exists() {
            Dictionary::from_path(&converted).map_err(invalid)?
        } else {
            let imported =
                import::import(&root.join("rime_ice.dict.yaml"), &cache).map_err(invalid)?;
            // 导入器用主词库名称命名；缓存保留它生成的完整 .qj。
            if imported.path != converted {
                std::fs::copy(&imported.path, &converted)?;
            }
            Dictionary::from_path(&converted).map_err(invalid)?
        };
        let mut english = String::new();
        for name in ["en_dicts/en.dict.yaml", "en_dicts/en_ext.dict.yaml"] {
            english.push_str(&import::to_tsv(&read(root, name)?).tsv);
        }
        let symbols = yaml(&read(root, "symbols_v.yaml")?)?["symbols"]
            .as_hash()
            .ok_or_else(|| invalid("missing symbols mapping"))?
            .iter()
            .filter_map(|(k, v)| Some((k.as_str()?.to_owned(), strings(v))))
            .collect();
        let phrases = table(&read(root, "custom_phrase.txt")?);
        let mixed = table(&read(root, "en_dicts/cn_en.txt")?);
        let radical_text = import::to_tsv(&read(root, "radical_pinyin.dict.yaml")?).tsv;
        let mut radicals: Vec<(String, String, u32)> = radical_text
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                Some((
                    fields
                        .nth(1)?
                        .chars()
                        .filter(|c| !c.is_whitespace() && *c != '\'')
                        .collect::<String>(),
                    line.split('\t').next()?.to_owned(),
                    fields.next().and_then(|f| f.parse().ok()).unwrap_or(1),
                ))
            })
            .collect();
        radicals.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.1.cmp(&b.1))
        });
        let mut readings = HashMap::new();
        for line in import::to_tsv(&read(root, "cn_dicts/8105.dict.yaml")?)
            .tsv
            .lines()
        {
            let mut fields = line.split('\t');
            if let (Some(word), Some(code)) = (fields.next(), fields.next()) {
                readings.entry(word.to_owned()).or_insert(code.to_owned());
            }
        }
        let mut pins = HashMap::new();
        for line in strings(&schema["pin_cand_filter"]) {
            if let Some((code, words)) = line.split_once('\t') {
                pins.insert(
                    code.to_owned(),
                    words.split_whitespace().map(str::to_owned).collect(),
                );
            }
        }
        let emoji_text = read(root, "opencc/emoji.txt")?;
        let mut emoji = String::new();
        for line in emoji_text.lines() {
            if let Some((word, values)) = line.split_once('\t') {
                let values: Vec<_> = values.split_whitespace().filter(|v| *v != word).collect();
                if !values.is_empty() {
                    emoji.push_str(&format!("{word}\t{}\n", values.join(" ")));
                }
            }
        }
        let mut corrections = HashMap::new();
        // corrector.lua 的数据表是字面字符串三元组，只读它的条目，不运行任何 Lua 代码。
        for line in read(root, "lua/corrector.lua")?.lines() {
            let quoted: Vec<_> = line.split('"').skip(1).step_by(2).collect();
            if line.trim_start().starts_with("[\"") && quoted.len() == 3 {
                corrections.insert(
                    quoted[0].to_owned(),
                    (quoted[1].to_owned(), quoted[2].to_owned()),
                );
            }
        }
        let keys = [
            ("date", "rq"),
            ("time", "sj"),
            ("week", "xq"),
            ("datetime", "dt"),
            ("timestamp", "ts"),
            ("datezh", "rqzh"),
            ("dateen", "rqen"),
        ];
        let date_keys = keys
            .into_iter()
            .map(|(kind, fallback)| {
                (
                    schema["date_translator"][kind]
                        .as_str()
                        .unwrap_or(fallback)
                        .to_owned(),
                    kind.to_owned(),
                )
            })
            .collect();
        Ok(Self {
            source: root.to_owned(),
            dictionary,
            english: WordList::parse(&english).map_err(invalid)?,
            emoji: EmojiTable::parse(&emoji)
                .map_err(|line| invalid(format!("invalid emoji line {line}")))?,
            symbols,
            phrases,
            mixed,
            pins,
            radicals,
            readings,
            corrections,
            reduced_english: strings(&schema["reduce_english_filter"]["words"])
                .into_iter()
                .collect::<HashSet<_>>(),
            long_count: schema["long_word_filter"]["count"]
                .as_i64()
                .unwrap_or(2)
                .clamp(0, 32) as usize,
            long_index: schema["long_word_filter"]["idx"]
                .as_i64()
                .unwrap_or(4)
                .clamp(1, 500) as usize
                - 1,
            reduced_index: schema["reduce_english_filter"]["idx"]
                .as_i64()
                .unwrap_or(2)
                .clamp(1, 500) as usize
                - 1,
            date_keys,
            lunar_key: schema["lunar"].as_str().unwrap_or("nl").to_owned(),
            lunar_template: schema["lunar_template"]
                .as_str()
                .unwrap_or("{干支年}{生肖}年{俗称农历月}{农历日}")
                .to_owned(),
            uuid_key: schema["uuid"].as_str().unwrap_or("uuid").to_owned(),
            uuid_cache: Default::default(),
        })
    }
}
