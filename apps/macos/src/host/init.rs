//! 启动：加载词库 / 语言模型 / 释义表 / 学习数据，建 Engine 与候选窗口，装进线程局部的 HOST。

use super::*;

/// 加载数据并建立单例。必须在主线程、在 IMKServer 建立之前调用。版本显示在菜单末行与「关于」页。
pub fn init(mtm: MainThreadMarker, info: &BundleInfo) -> Result<(), HostError> {
    let version = info.version.as_str();
    let settings = Settings::load();
    let started = std::time::Instant::now();
    // 打包过的 .qj 直接映射；没有就解析 TSV（样例词库）
    let dictionary = Dictionary::from_path(
        paths::resource("dict.qj").or_else(|_| paths::resource("dict.tsv"))?,
    )?;
    let dictionary_ms = started.elapsed().as_millis();
    let languages: Vec<Language> = GLOSSARY_LANGUAGES
        .into_iter()
        .filter(|language| glossary_path(*language).is_ok())
        .collect();
    // 配置里的学习语言没有对应释义表时退回第一种有的；一种都没有就按老样子报错
    let learning_language = settings
        .config()
        .general
        .learning_language
        .parse::<Language>()
        .ok()
        .filter(|language| languages.contains(language))
        .or_else(|| languages.first().copied())
        .unwrap_or(Language::English);
    let glossary = load_glossary(learning_language)?;
    let learner = match paths::user_data_dir() {
        Some(dir) => load_learner(&dir),
        None => FrequencyLearner::default(),
    };
    // 英文词表可选：没有就不出英文候选
    let english = paths::resource("english.tsv")
        .ok()
        .map(WordList::from_path)
        .transpose()?;
    tracing::info!(
        entries = dictionary.len(),
        glosses = glossary.len(),
        english = english.as_ref().map_or(0, WordList::len),
        learned = learner.len(),
        dictionary_ms,
        "数据加载完成"
    );
    let mut engine = Engine::new(dictionary)
        .with_translator(Box::new(glossary))
        .with_learner(Box::new(learner));
    // 输入统计（打了多少字）：与学习数据同目录；没有数据目录就只在内存里数
    if let Some(dir) = paths::user_data_dir() {
        // 词汇等级表（levels-en.tsv / levels-ja.tsv）随包可选：有就按级统计
        let mut vocabulary = VocabularyBook::open(dir.join(VOCABULARY_FILE));
        for language in GLOSSARY_LANGUAGES {
            let Ok(path) = paths::resource(&format!("levels-{}.tsv", language.code())) else {
                continue;
            };
            match LevelTable::from_path(&path) {
                Ok(table) => vocabulary = vocabulary.with_levels(language, table),
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "词汇等级表读不了，不分级")
                }
            }
        }
        engine = engine
            .with_usage_meter(Box::new(UsageStats::open(dir.join(USAGE_FILE))))
            .with_vocabulary_tracker(Box::new(vocabulary));
    }
    // 附加词库：随包的领域词库 + 用户目录 dicts/ 下的文件
    engine.set_extra_dictionaries(extra_dictionaries::load(
        paths::bundled_dicts_dir().as_deref(),
        paths::dicts_dir().as_deref(),
        &settings.config().dictionaries,
    ));
    // 英文候选的中文释义（英→中）可选：没有这张表英文候选右侧就留空
    if let Ok(path) =
        paths::resource("glossary-zh.qj").or_else(|_| paths::resource("glossary-zh.tsv"))
    {
        match Glossary::from_path(Language::Chinese, &path) {
            Ok(glossary) => {
                tracing::info!(glosses = glossary.len(), "英→中释义表已加载");
                engine = engine.with_english_translator(Box::new(glossary));
            }
            Err(error) => tracing::warn!(%error, "英→中释义表加载失败"),
        }
    }
    if let Some(words) = english {
        engine = engine.with_english(words);
    }
    if let Some(table) = load_emoji_tables(&["emoji-zh.tsv", "emoji-en.tsv"]) {
        tracing::info!(words = table.len(), "emoji 表已加载");
        engine = engine.with_emoji(table);
    }
    // 语言模型可选：没有就退化成一元词频整句；打包过的 lm.qj 优先
    let model = if let Ok(packed) = paths::resource("lm.qj") {
        Some(BigramModel::from_path(&packed)?)
    } else if let (Ok(unigram), Ok(bigram)) = (
        paths::resource("lm-unigram.tsv"),
        paths::resource("lm-bigram.tsv"),
    ) {
        Some(BigramModel::from_paths(&unigram, &bigram)?)
    } else {
        None
    };
    if let Some(model) = model {
        tracing::info!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            total_ms = started.elapsed().as_millis(),
            "语言模型已加载"
        );
        engine = engine.with_language_model(Box::new(model));
    }
    let window = CandidateWindow::new(mtm);
    let indicator = ModeIndicator::new(mtm);
    let menu = InputMenu::new(mtm, version);
    indicator.set_menu(&menu.ns_menu());
    let preferences = PreferencesWindow::new(mtm, &languages, version, &info.build);
    let monitor = PredictMonitor::new(mtm);
    let watch = ConfigWatch::new(mtm);
    HOST.with(|host| {
        *host.borrow_mut() = Some(Host {
            engine,
            window,
            indicator,
            menu,
            preferences,
            settings,
            watch,
            last_flush: std::time::Instant::now(),
            applied_predict: PredictConfig::default(),
            applied_dictionaries: DictionariesConfig::default(),
            dictionary_list: Vec::new(),
            learning_language,
            languages,
            version: version.to_owned(),
            build: info.build.clone(),
            page_size: 9,
            cloud_slots: 2,
            page_keys: qingjian_platform::DEFAULT_PAGE_KEYS,
            translation_keys: ShortcutConfig::default().translation_keys(),
            delete_keys: ShortcutConfig::default().delete_keys(),
            status: None,
            input_log_enabled: None,
            translate_keys: KeyCombo::TRANSLATE_DEFAULT,
            translation: None,
            notice: None,
            preedit_mode: PreeditMode::default(),
            english_candidates: true,
            apps: AppsConfig::default(),
            monitor,
            cloud_test: None,
            cloud_test_monitor: CloudTestMonitor::new(mtm),
            session: Session::default(),
            sentence: None,
            anchor: NSRect::ZERO,
        })
    });
    // 配置里的开关走和菜单 / 设置窗口 / 热加载同一条通路
    with(|host| host.apply_config(false));
    Ok(())
}

/// 读用户的学习数据。格式坏掉的行学习 crate 自己跳过；真读不了（权限、坏盘）就退回只在内存里学，
/// 输入法照常启动，也不会拿空表覆盖用户的文件。学习数据出问题不能让输入法起不来。
pub(super) fn load_learner(dir: &std::path::Path) -> FrequencyLearner {
    let path = dir.join("user.tsv");
    match FrequencyLearner::from_path(&path) {
        Ok(learner) => learner,
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "学习数据读取失败，本次只在内存里学习");
            FrequencyLearner::default()
        }
    }
}

/// 读一本词库的显示信息；文件坏了标 `broken`。
pub(super) fn dictionary_info(
    stem: String,
    path: PathBuf,
    builtin: bool,
    enabled: bool,
) -> DictionaryInfo {
    match Dictionary::from_path(&path) {
        Ok(dictionary) => DictionaryInfo {
            name: dictionary
                .metadata()
                .map_or(stem.clone(), |m| m.name.clone()),
            entries: dictionary.len(),
            license: dictionary
                .metadata()
                .map_or(String::new(), |m| m.license.clone()),
            enabled,
            broken: false,
            stem,
            builtin,
            path,
        },
        Err(_) => DictionaryInfo {
            name: stem.clone(),
            entries: 0,
            license: String::new(),
            enabled,
            broken: true,
            stem,
            builtin,
            path,
        },
    }
}

/// 把 `stem` 加进（`present` 为真）或移出字符串列表；没变化返回 `None`。
pub(super) fn toggle_membership(
    mut list: Vec<String>,
    stem: &str,
    present: bool,
) -> Option<Vec<String>> {
    let has = list.iter().any(|d| d == stem);
    if present && !has {
        list.push(stem.to_owned());
    } else if !present && has {
        list.retain(|d| d != stem);
    } else {
        return None;
    }
    Some(list)
}

pub(super) fn string_array(items: &[String]) -> toml_edit::Array {
    items.iter().map(String::as_str).collect()
}

/// 把包里有的 emoji 表（中文、英文）合成一张；一张都没有返回 `None`，坏了的只记日志跳过。
pub(super) fn load_emoji_tables(names: &[&str]) -> Option<EmojiTable> {
    let mut merged: Option<EmojiTable> = None;
    for name in names {
        let Ok(path) = paths::resource(name) else {
            continue;
        };
        match EmojiTable::from_path(&path) {
            Ok(table) => match &mut merged {
                Some(all) => all.merge(table),
                None => merged = Some(table),
            },
            Err(error) => tracing::warn!(%error, name, "emoji 表加载失败，跳过"),
        }
    }
    merged
}

pub(super) fn glossary_path(language: Language) -> Result<PathBuf, HostError> {
    paths::resource(&format!("glossary-{}.qj", language.code()))
        .or_else(|_| paths::resource(&format!("glossary-{}.tsv", language.code())))
}

/// 随包释义表叠上用户目录的个人释义表（`user-glossary-<语言>.tsv`，释义兜底写入、可手改）。
pub(super) fn load_glossary(language: Language) -> Result<LayeredTranslator, HostError> {
    let bundled = Glossary::from_path(language, glossary_path(language)?)?;
    let personal = match paths::user_data_dir() {
        Some(dir) => PersonalGlossary::open(
            language,
            dir.join(format!("user-glossary-{}.tsv", language.code())),
        ),
        None => PersonalGlossary::in_memory(language),
    };
    if !personal.is_empty() {
        tracing::info!(
            language = language.code(),
            entries = personal.len(),
            "个人释义表已加载"
        );
    }
    Ok(LayeredTranslator::new(bundled, personal))
}
