---
title: Dictionaries
order: 4
description: "The bundled core and idiom dictionaries; downloading the general dictionary, topic dictionaries and glossaries; importing your own pinyin and Wubi dictionaries, turning them on and off, and removing them."
---

## Bundled dictionaries

The core dictionary and the idiom dictionary ship with Qingjian; both are listed under 设置 → 词库 → 随包词库 (Settings → Dictionaries → Bundled dictionaries). The core dictionary has about 340,000 entries (130,000 common words plus 210,000 supplementary ones); the idiom dictionary is on by default. Both can be turned on or off.
With the core dictionary off, its words no longer appear but single characters can still be typed; words, including those in sentences, then come only from dictionaries you imported or downloaded. This is for when you want to switch entirely to your own dictionaries. Other dictionaries are installed from 下载 (Download) below:

- **通用词库 (General dictionary)**: 1.5 million entries of 2 to 30 characters, compiled from public text and still being cleaned up as it's used.
  When readings are the same, words from the core dictionary come before it; its long words also don't show up before you've finished typing: `anzhao` doesn't give 按照规定, only the full input does.
- **领域词库 (Topic dictionaries)**: law, medicine, place names, famous lines of poetry, IT, finance, food, animals, cars, historical figures.
  Topic dictionaries only decide whether a word exists; they don't change how sentences are built.

The core dictionary includes 36,000 rare characters (extended characters outside the standard character table, plus characters rarely used in Taiwan). They can be typed in pinyin, come last among characters with the same sound, and aren't used in sentences:
`fiao` gives 覅, `biangbiangmian` gives 𰻝𰻝面. Characters missing from the system fonts are shown as boxes.

## Downloading dictionaries

设置 → 词库 → 下载 (Settings → Dictionaries → Download) lists the dictionaries and glossaries you can download from the Qingjian dictionary repository; 词库 ｜ 释义表 (Dictionaries | Glossaries) at the top switches between the two; dictionaries are grouped by language when there is more than one. Each shows a description, its author, size and license; click an author's name, when it is a link, to open their homepage. Ones put together by others are marked 第三方 (Third-party) next to the name.
The list is fetched once when Settings opens; when offline, the last list fetched is shown, with the time it was fetched above it. Click 刷新 (Refresh) to fetch it again. 下载 (Download) installs a dictionary into your data folder; it then appears under 我的词库 (My dictionaries) alongside imported ones, turned on by default; its checkbox turns it on or off, and it can be removed.
When the repository has a newer version, the button changes to 更新 (Update). If you start several downloads, they run one after another; the ones waiting show 排队中 (Queued) and can be cancelled.

When you upgrade from an older version, the domain dictionaries and the general dictionary that used to ship with Qingjian and that you had turned on, plus the glossary your learning language uses, are downloaded again one by one the next time you open Settings, so things work as they did before the upgrade.

Entries under 释义表 (Glossaries) are the tables behind the glosses beside candidates (learning languages such as Japanese or Spanish, and the Chinese beside English candidates). Once downloaded, the input method switches to them without a restart. They don't appear under 我的词库 (My dictionaries), and a downloaded table takes precedence over the bundled one for the same language.

## Importing your own dictionaries

设置 → 词库 → 导入词库 (Settings → Dictionaries → Import dictionary) lets you select several files at once and accepts five kinds:

- **Text dictionaries** (`.tsv` / `.txt`): each line is "word, pinyin, frequency", separated by tabs, with spaces between pinyin syllables: `青简	qing jian	100`. Pinyin can carry numeric tones (`ni3 hao3`, with 5 or 0 for the neutral tone) or not; typing works the same either way. Write ü as `v`; `lue` / `nue` are accepted too.
- **Rime dictionaries** (`.dict.yaml`): the name and entries (each line "word, pinyin, optional weight") are read. Other dictionaries it references, custom column orders and schema settings aren't supported. Lines with only a word and no pinyin get readings as described in the next item.
- **Word lists without readings** (`.txt`, or Rime dictionaries with no pinyin): one word per line, optionally "word, frequency". Readings are filled in from the bundled core and topic dictionaries:
  a reading written in the list wins, then the whole word's reading in the dictionaries, then readings from splitting it by dictionary words (`中华人民共和国` uses 中华, 人民 and 共和国), and finally the most common reading for each remaining character.
  Words whose characters aren't in the bundled dictionaries are skipped; the result says how many were skipped and shows the first few.
- **Sogou cell dictionaries** (`.scel`): dictionaries downloaded from Sogou's dictionary site. Words and pinyin are read; they carry no frequencies (each word counts as 100), and the name comes from the file name.
- **Qingjian dictionaries** (`.qj`, or compressed `.qjz`): dictionary files made by others, with their own name and license. A `.qjz` is unpacked during import; there's no need to decompress it first.

- Imported dictionaries are listed below the bundled ones with their name, entry count and license, and are turned on automatically; the input method usually starts using them within about a second, no restart needed.
- One file failing doesn't affect the others; the result for each file is shown when done. Files with no usable entries can't be imported.
- Each can be turned on or off on its own; 移除 (Remove) only moves the file aside rather than deleting it. To delete it for good, see [Uninstalling](../help/uninstall.md#Clearing-only-some-data).
- 导出… (Export…) saves one as TSV (word, pinyin, frequency; known tones as digits, with the name and license in comment lines at the top), so you can edit it and import it again. Downloaded dictionaries can be exported too.
- Re-importing a dictionary with the same name replaces the old one. Rime dictionaries need to go through 导入词库 (Import dictionary) to be converted; putting them straight into Qingjian's data folder doesn't work.

On Android, dictionaries are imported in the Qingjian app; see [Android: vocabulary book and settings](../platforms/android-settings.md#Dictionaries).

## Importing Wubi dictionaries

Wubi dictionaries are also imported through 导入词库 (Import dictionary). The second column decides: if the second column of multi-character words is letters run together (`gant`), it's a Wubi dictionary; if it has spaces (`kai fa`), it's a pinyin dictionary.
Files whose format can't be recognized give an error right away, rather than reporting success and then not being typeable. A file with only single characters can't be told apart, so it's imported as a pinyin dictionary.

Each line of a Wubi dictionary is "word, code, frequency", separated by tabs; the frequency can be omitted, and lines starting with `#` are comments. Rime Wubi dictionaries work too:

```text
青简	getu	100
```

- Imported words are used together with the bundled Wubi code table; you don't need to replace the whole table. Words with the same code are ranked by frequency, so give your words a higher frequency if you want them first.
- The dictionary belongs to the scheme selected under 设置 → 输入 → 形码 (Settings → Input → Shape-based) when you import it (Wubi, if shape-based input is off). It's inactive under other shape-based schemes and works again when you switch back.
- The list shows the scheme name after the dictionary name, for example 我的五笔词（五笔（86））, and it can be turned on, off and removed like a pinyin dictionary.
- Supported on macOS and Windows; Linux and Android can't import Wubi dictionaries yet.

## Blocked words

Imported dictionaries sometimes drag in words you never want to see — some third-party packs fill brand words like "搜狗" with a very high frequency, so even a typo like `sougo` puts them on the first page. One line in the config file drops them:

```toml
[dictionaries]
blocked = ["搜狗"]
```

Matching is by **substring**: blocking "搜狗" also removes "搜狗拼音", "搜狗输入法" and the rest.
Custom phrases are left alone — those are your own text.

Save the file and the input method reloads it within about a second; no restart needed. There is no
setting page for this yet, so edit the config file for now. Supported on macOS, Windows and Linux; not yet on Android.