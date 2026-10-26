---
title: Dictionaries
order: 4
description: "The bundled core dictionary, general dictionary and 11 topic dictionaries; importing your own pinyin and Wubi dictionaries, turning them on and off, and removing them."
---

## Bundled dictionaries

The core dictionary has 130,000 entries, and a general dictionary and 11 topic dictionaries are bundled too. Turn each on or off under 设置 → 词库 (Settings → Dictionaries):

- **通用词库 (General dictionary)**: 1.5 million entries of 2 to 30 characters, compiled from public text and still being cleaned up as it's used. On by default.
  When readings are the same, words from the core dictionary come before it; its long words also don't show up before you've finished typing: `anzhao` doesn't give 按照规定, only the full input does.
- **领域词库 (Topic dictionaries)**: law, medicine, place names, idioms, famous lines of poetry, IT, finance, food, animals, cars, historical figures. Only idioms are on by default.
  Topic dictionaries only decide whether a word exists; they don't change how sentences are built.

The core dictionary includes 36,000 rare characters (extended characters outside the standard character table, plus characters rarely used in Taiwan). They can be typed in pinyin, come last among characters with the same sound, and aren't used in sentences:
`fiao` gives 覅, `biangbiangmian` gives 𰻝𰻝面. Characters missing from the system fonts are shown as boxes.

## Importing your own dictionaries

设置 → 词库 → 导入词库 (Settings → Dictionaries → Import dictionary) lets you select several files at once and accepts four kinds:

- **Text dictionaries** (`.tsv` / `.txt`): each line is "word, pinyin, frequency", separated by tabs, with spaces between pinyin syllables: `青简	qing jian	100`. Pinyin can carry numeric tones (`ni3 hao3`, with 5 or 0 for the neutral tone) or not; typing works the same either way. Write ü as `v`; `lue` / `nue` are accepted too.
- **Rime dictionaries** (`.dict.yaml`): the name and entries (each line "word, pinyin, optional weight") are read. Other dictionaries it references, custom column orders and schema settings aren't supported. Lines with only a word and no pinyin get readings as described in the next item.
- **Word lists without readings** (`.txt`, or Rime dictionaries with no pinyin): one word per line, optionally "word, frequency". Readings are filled in from the bundled core and topic dictionaries:
  a reading written in the list wins, then the whole word's reading in the dictionaries, then readings from splitting it by dictionary words (`中华人民共和国` uses 中华, 人民 and 共和国), and finally the most common reading for each remaining character.
  Words whose characters aren't in the bundled dictionaries are skipped; the result says how many were skipped and shows the first few.
- **Qingjian dictionaries** (`.qj`): dictionary files made by others, with their own name and license.

- Imported dictionaries are listed below the bundled ones with their name, entry count and license, and are turned on automatically; the input method usually starts using them within about a second, no restart needed.
- One file failing doesn't affect the others; the result for each file is shown when done. Files with no usable entries can't be imported.
- Each can be turned on or off on its own; 移除 (Remove) only moves the file aside rather than deleting it. To delete it for good, see [Uninstalling](../help/uninstall.md#Clearing-only-some-data).
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
