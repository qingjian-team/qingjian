---
title: Editing the configuration file
order: 1
description: "All of Qingjian's settings live in one text configuration file: where it is and how to open it, how changes take effect, what happens when it contains a mistake, how its entries map to the settings window, and common entries on Linux."
---

Anything you can change in the settings window doesn't require touching the configuration file. This page is for people who want to edit the configuration file directly, or need options the settings window doesn't have.

## Location and opening

All settings are kept in one TOML text file, `config.toml`, with a note for every entry:

- **macOS**: `~/Library/Application Support/Qingjian/config.toml`
- **Windows**: `%APPDATA%\Qingjian\config.toml`
- **Linux**: `~/.config/qingjian/config.toml` (under `qingjian/` in `XDG_CONFIG_HOME` if that's set)

在编辑器中打开 (Open in editor) under 设置 → 高级 → 配置文件 (Settings → Advanced → Configuration file) opens it directly, and 打开数据目录 (Open data folder) opens the folder it's in.

## How changes take effect

- Changes take effect within a second of saving, with no need to switch input methods again. On Linux, `preedit` and `input_log` under `[general]` are the exception: restart Qingjian after changing them.
- If the file contains a mistake, the last valid settings stay in use and the input method keeps running; on macOS the input method menu shows an extra grey line describing the error. Fix it as described and save.
- A mistake in one custom phrase (a weight outside 1–999, content that isn't text, the same code and content as an earlier entry and so on) skips only that phrase; the rest still apply, and the runtime log notes which entry was skipped and why. When you save phrases in the settings window, skipped entries are left in the file as they are.
- Each section (such as `[general]`) is written only once; put the entries you want to change under the existing section.

## Entries and the settings window

| Entry | Settings window | Notes |
|---|---|---|
| `[general] scheme` | 输入 → 拼音 (Input → Pinyin) | `pinyin` (full pinyin), `xiaohe`, `ziranma`, `microsoft`, `sogou`, `abc`, `xiaolang`, `shoudao`, `jiajia`, `ziguang`, `zhuyin`, or the folder name of a custom scheme |
| `[general] hold_selections` | 输入 → 分次选词先不上屏 (Input → Keep partial picks until done) | `true` / `false` |
| `[aux_code] enabled`, `scheme` | 输入 → 辅码 (Input → Auxiliary code) | `scheme` is the code table's file name or the scheme's folder name; the bundled stroke code is `stroke` |
| `[general] aux_code_keep_empty` | 输入 → 码删空后留在辅码 (Input → Stay in auxiliary code when empty) | `false` leaves as soon as the code is empty |
| `[general] traditional_variant` | 候选 → 输出字形 (Candidates → Output characters) | `off`, `taiwan`, `hongkong`, `standard`; `traditional = true` from 0.2.1 and earlier is read as `taiwan` |
| `[general] emoji`, `kaomoji`, `symbols` | 候选 → emoji / 颜文字 / 符号 (Candidates → emoji / Kaomoji / Symbols) | `0`–`3`, default 3 / 1 / 1; the older `emoji = true / false` means 3 / 0 |
| `[general] learning` | 候选 → 学习输入习惯 (Candidates → Learn typing habits) | `true` / `false` |
| `[general] learning_language` | 候选 → 译文 (Candidates → Gloss) | `off` hides glosses |
| `[general] english_in_chinese` | 候选 → 打拼音时也给英文词 (Candidates → English words while typing pinyin) | `true` / `false` |
| `[model] enabled` | 候选 → 本地整句模型 (Candidates → Local sentence model) | `true` / `false` |
| `[model] name`, `zhiyi_tier` | 实验功能 (Experimental features) → which model, Zhiyi's level | `name`: `tongbian` (default), `zhiwei`, `zhiyi`; a model that isn't downloaded falls back to `tongbian`. `zhiyi_tier`: `saver`, `balanced` (default), `accurate` |
| `[model] context_memory_seconds` | Configuration file only | When an app doesn't share the text before the cursor, how long what you just typed counts as context; default 60, 0 means always |
| `[general] english_mode` | 中英文 → 英文模式 (Chinese/English → English mode) | `false` keeps it in Chinese |
| `[shortcut] switch_mode` | 中英文 → 切换键 (Chinese/English → Switch key) | A list, such as `["shift", "control"]` |
| `[general] default_english`, `remember_mode_per_app` | 中英文 → 默认英文模式, 按应用记住中英模式 (Chinese/English → Start in English mode, Remember mode per app) | `true` / `false` |
| `[general] shift_letter` | 中英文 → Shift + 字母 (Chinese/English → Shift + letter) | `passthrough` (to the app, default) or `compose` (join the input) |
| `[general] punct_commits` | 中英文 → 标点先上屏候选 (Chinese/English → Punctuation commits the candidate first) | `true` / `false` |
| `[general] preedit` | 外观 → 拼音显示位置 (Appearance → Pinyin position) | `both`, `inline`, `window` |
| `[general] page_keys` | 快捷键 → 翻页键 (Shortcuts → Paging keys) | `"[]"`, `",."`, `"-="` |
| `[shortcut] extra_page_keys` | 快捷键 → 额外翻页键 (Shortcuts → Extra paging keys) | A list, such as `["[]", ",.", "-="]`; empty by default |
| `[shortcut] highlight_keys` | 快捷键 → 高亮移动键 (Shortcuts → Highlight keys) | Up/down or left/right |
| `[shortcut] toggle_punctuation` | 快捷键 → Ctrl + . 切换全半角标点 (Shortcuts → Ctrl + . switches punctuation width) | An empty string turns it off, `"ctrl+."` is Ctrl + full stop; modifiers plus a letter, digit or full stop also work, such as `"ctrl+shift+."` |
| `[shortcut] translate_selection` | 快捷键 → 翻译选中文字 (Shortcuts → Translate selected text) | Such as `"alt+shift+t"` |
| `[shortcut] predict` | 快捷键 → 按需 AI 联想 (Shortcuts → On-demand AI suggestions) | The full key combination; the settings window only changes the modifiers |
| The `[predict]` section | 云服务 (Cloud services) | See below |
| `[general] input_log` | 高级 → 记录输入日志 (Advanced → Record input log) | `true` / `false` |

### The app list for English candidates

Which apps get candidates in English mode is decided by the list in the `apps` part of the configuration file (when 设置 → 中英文 → 只在写字的应用里给 (Settings → Chinese/English → Only in writing apps) is on). Each entry in the list has a note; add or remove apps as described.

## AI services

The `[predict]` section corresponds to 设置 → 云服务 (Settings → Cloud services):

```toml
[predict]
enabled = true
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
api_key = "your API key"
```

- `automatic`: automatic suggestions; with `false`, requests are made only when you press the on-demand suggestion key combination.
- `debounce_ms`: pause before request, default 300. `min_interval_ms`: minimum interval between requests, default 3000.
- `slots`: how many slots at the end of the first page AI candidates take, 0 to 4.
- A key entered in the settings window is stored separately in `.env` in the data folder, not in `config.toml`. On Linux you can also set the `QINGJIAN_API_KEY` environment variable before starting Qingjian.
- Changing the switch, address, model or key switches over automatically and cancels old requests.

If a Linux configuration upgraded from an earlier version still has the old translation shortcut, change it to `translate_selection = "alt+shift+t"` and save; installing updates doesn't overwrite settings you've written.

## Linux

Linux has the Qingjian settings app, and you can also edit the configuration file directly. A few common entries:

- Double pinyin: `scheme = "xiaohe"` under `[general]`.
- Traditional Chinese: `traditional_variant = "taiwan"` under `[general]`; configurations upgraded from 0.2.1 or earlier don't have this line, so add it under `[general]`.
- Paging keys: `page_keys = "-="` under `[general]`. Changing Fcitx5's global candidate paging hotkeys doesn't replace it.
- No glosses: `learning_language = "off"` under `[general]`.
- No input records at all (for example when Fcitx4 can't recognize private input fields): `input_log = false` and `learning = false` under `[general]`, then restart Qingjian.

```toml
[general]
scheme = "xiaohe"
page_keys = "-="
```
