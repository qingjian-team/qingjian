---
title: Data files and logs
order: 2
description: "Where Qingjian's data folder is, what each file records, where the runtime logs are, and using your own local sentence model."
---

What Qingjian records is summarized in [Privacy](../help/privacy.md#What-it-records-on-your-device); this page lists the actual files.

## Data files

Where the data folder is:

- **macOS**: `~/Library/Application Support/Qingjian/` (in Finder, press `⇧⌘G` and enter this path).
- **Windows**: `%APPDATA%\Qingjian` (type this path straight into the File Explorer address bar).
- **Linux**: `~/.local/share/qingjian`; the configuration file is in `~/.config/qingjian/`. If `XDG_DATA_HOME` / `XDG_CONFIG_HOME` are set, `qingjian` under them is used.

打开数据目录 (Open data folder) under 设置 → 高级 (Settings → Advanced) opens it directly. The files are all plain text and can be opened any time:

| File | Contents | Includes what you typed |
|---|---|---|
| `config.toml` | Settings; see [Editing the configuration file](config-file.md) | No |
| `.env` | The AI service key, only once you've entered one; readable only by your user account | No |
| `user.tsv` | How many times each word was picked | Words you picked |
| `user-words.tsv` | Words you created and AI candidates you picked | Words you picked |
| `user-choices.tsv` | Words picked for a given stretch of pinyin | Pinyin and words you picked |
| `user-demoted.tsv` | Words demoted from the candidate right-click menu (which letters, which word, how many times) | Pinyin and words |
| `user-days.tsv` and other `*-days.tsv` | When the entries in the tables above were last used | No |
| `user-english.tsv` | English words committed as typed | English words |
| `user-typos.tsv` | Spelling corrections you accepted (what you typed and what you meant) | Pinyin syllables |
| `user-ngram.tsv` | Which words follow which | Words typed in sequence |
| `user-glossary-<language>.tsv` | Glosses filled in by AI | Words and glosses |
| `vocab-log.tsv` | The vocabulary book timeline: one line per commit of Chinese with a gloss (time, Chinese, gloss, reading) | Chinese and translations |
| `user-vocab.tsv` | The vocabulary book word list: how often each gloss was seen / committed / output, the latest Chinese and example sentence, and whether it's marked as known | Glosses, Chinese and example sentences |
| `usage-hourly.tsv` | Hourly counts of characters, words, commits, first choices, keystrokes and typing time — numbers only | No |
| `usage.tsv` | Daily typing statistics from before 0.2.2; read only, no longer written | No |
| `api-usage.json`, `api-usage-hourly.tsv` | AI service request counts and tokens — numbers only | No |
| `update.json` | When updates were last checked and the new version found | No |
| `input-log.jsonl` | The input log; see [Privacy](../help/privacy.md#Input-log) | Keystrokes and committed text |
| `dicts/` | Imported dictionaries; removed ones are in `dicts/removed/` | No |
| `codes/` | Imported auxiliary code tables | No |
| `themes/` | Custom themes; see [Custom themes](custom-themes.md) | No |
| `schemes/` | Custom double pinyin, shape-based and auxiliary code schemes; see [Custom schemes and code tables](custom-schemes.md) | No |
| `models/` | Local sentence models you've added; see below | No |

These files are replaced whole each time, so even if the input method crashes it never leaves a half-written file.

To start your typing habits over, delete the tables whose names start with `user` (except `user-glossary-*` and `user-vocab.tsv`); clear the vocabulary book on the 生词本 (Vocabulary book) page in Settings.

## Runtime logs

- **macOS**: `~/Library/Logs/Qingjian/`; the input method's logs are "qingjian.log.date" and the settings window's are "qingjian-settings.date.log".
- **Windows**: `%LOCALAPPDATA%\Qingjian\logs`; the background program ("server.date.log"), the input method part inside each app ("tsf.date.log") and the settings window ("qingjian-settings.date.log") all log to this one folder.
- **Linux**: `~/.local/state/qingjian/logs` (under `XDG_STATE_HOME` if that's set).

打开日志目录 (Open log folder) under 设置 → 高级 → 日志文件 (Settings → Advanced → Log files) opens it directly; 打包到桌面 (Bundle to desktop) packs the logs together with your settings (keys scrubbed) into a zip to attach to a report; see [Reporting problems](../help/feedback.md).
One file per day, kept for 7 days. What you type isn't recorded by default; only with 详细日志 (Verbose logging) on is each keystroke recorded.

## Using your own local sentence model

The local sentence model (Hanzhang Tongbian) is bundled. To use a model you've trained yourself, put `hanzhang-tongbian-small.qjm` into `models/hanzhang-tongbian/` in the data folder; after the input method restarts, it's used first:

- **macOS**: `~/Library/Application Support/Qingjian/models/hanzhang-tongbian/`
- **Windows**: `%APPDATA%\Qingjian\models\hanzhang-tongbian\`
- **Linux**: `~/.local/share/qingjian/models/hanzhang-tongbian/`

- Hanzhang Zhiwei (`hanzhang-zhiwei-small.qjm`, in `models/hanzhang-zhiwei/`) is no longer bundled. Download it under 设置 → 实验功能 (Settings → Experimental features), or put it in that folder yourself and turn it on there (`[model] name = "zhiwei"`); it uses the few dozen characters before the cursor to reorder candidates. Zhiyi is downloaded to `models/zhiyi/` (`models/zhiyi-onnx/` on Windows).
- The `model-p2c/` and `model/` folders used by older versions can still be read; for the same model, the new folder takes priority.
- The three files exported directly, `model.safetensors`, `config.json` and `vocab.json`, are recognized too; if a folder has both kinds, `.qjm` is used first.
