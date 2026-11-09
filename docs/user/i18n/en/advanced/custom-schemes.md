---
title: Custom schemes and code tables
order: 4
description: "Writing your own double pinyin, shape-based and auxiliary code schemes, and the file format of auxiliary code tables. Supported on macOS and Windows."
---

Custom schemes go in the `schemes` folder of the [data folder](data-files.md#Data-files), one subfolder per scheme. The folder name is the scheme's identifier: only lowercase letters, digits, `-` and `_`, and it can't be the same as a built-in scheme name (such as `xiaohe` or `pinyin`).
The subfolder must contain a `scheme.toml`; a folder without one isn't a scheme. The following fields are common to all kinds of scheme and can all be omitted; when sharing a scheme, fill them in so others know whose it is and whether they may redistribute it:

```toml
name = "My scheme"     # The name shown in Settings and the status bar; defaults to the folder name
author = "Someone"
version = "1.0"
description = "A sentence or two about what this scheme is"
license = "CC-BY-4.0"
```

Supported on macOS and Windows; not yet on Linux.

## Custom double pinyin schemes

In `scheme.toml`, write:

```toml
type = "shuangpin"     # Required: marks this as a double pinyin scheme
name = "My Xiaohe"
base = "xiaohe"        # The built-in scheme to start from; write only the keys you change. Omit to start from scratch

[finals]               # Final keys: key = final; use a list when one key holds several finals
k = "uai"
";" = "ing"            # ; needs quotes

[initials]             # Retroflex initials: key = zh / ch / sh
v = "zh"

[zero]                 # Zero initials: syllable = two-key spelling; use a list for several spellings, the first being the main one
ang = ["ah", "ag"]
```

- `base` can be `xiaohe` (Xiaohe), `ziranma` (Ziranma), `microsoft` (Microsoft), `sogou` (Sogou), `abc` (Zhineng ABC), `xiaolang` (Xiaolang), `shoudao` (Shoudao), `jiajia` (Pinyin Jiajia) or `ziguang` (Ziguang).
- In all three tables, each entry replaces the base's entry for the same key (or syllable, for zero initials) as a whole. To move a final to another key, rewrite the original key as well, or both keys will type it; an empty value (`k = ""`) removes the final from that key.
- Initial keys hold only zh, ch and sh; the other initials are the letters themselves, so they can't be placed on keys like b, p or m that are already initials.
- `;` can only be a final key (the second key), like ing in Microsoft double pinyin: after a lone initial it's pinyin, otherwise it's still punctuation.
- A scheme with mistakes (a key that isn't a letter, a final that doesn't exist, a zero-initial spelling that isn't two keys) doesn't appear in Settings; if only some syllables can't be typed, it still works, and the [runtime log](data-files.md#Runtime-logs) lists which ones.

Once it's in place, switch back to the settings window and choose it under 设置 → 输入 → 拼音 (Settings → Input → Pinyin); built-in double pinyin schemes come first, followed by your own. Fuzzy pinyin, 输入框显示按键 (Show keys in the input box), 候选窗显示按键 (Show keys in the candidate window) and key hints work as with the built-in schemes.

## Custom shape-based schemes

```text
schemes/
└── huma/
    ├── scheme.toml   (scheme description, required)
    └── table.tsv     (code table)
```

The code table is plain text, each line "word, code, frequency", separated by tabs; the frequency can be omitted, and lines starting with `#` are comments:

```text
一	ggll	90000
开发	gant	900
```

- Codes can only be English letters and are case-insensitive; lines whose codes contain other symbols are skipped.
- Codes for multi-character words have to be written out in full in the table; Qingjian doesn't derive word codes from character codes.
- The full code length is the most common code length in the table (4 for Wubi); a few longer codes don't affect it.

Besides the common fields, `scheme.toml` can also contain:

```toml
table = "table.tsv"   # Code table file name; defaults to table.tsv
auto_commit = true    # Default for auto-committing unique full codes; on if omitted
top = true            # Default for commit on next code; on if omitted
code_length = 4       # How many letters make a full code; defaults to the most common code length in the table
```

`auto_commit` and `top` are only defaults; whatever you've changed in Settings takes priority. An empty `scheme.toml` works too.

Once it's in place, reopen Settings and choose it under 设置 → 输入 → 形码 (Settings → Input → Shape-based). A scheme whose folder is named `wubi86` replaces the bundled Wubi, which you can use to swap in a different Wubi code table.
If you just want to add a few words of your own without replacing the code table, see [Importing Wubi dictionaries](../personalize/dictionaries.md#Importing-Wubi-dictionaries).

## Auxiliary code tables

The smallest code table you can import from 设置 → 输入 → 码表 → 导入码表 (Settings → Input → Code tables → Import code table), `my.dict.yaml` (each line of the body is "word, tab, code"):

```yaml
---
name: My code table
version: "1"
...

开发	kf
开放	kf
鹤	hn
```

- Codes use only lowercase letters a–z; a word written on several lines has several codes, and the code column can also hold several codes separated by spaces.
- Between `---` and `...` in the header you can write `name` (the name shown in the code table list), `version`, `columns` (column names) and `import_tables` (merged tables); plain text with no header, just "word, tab, code" on each line, is accepted too.
- Third-party code tables (such as Xiaohe's shape code) are obtained by users from their official sources, arranged in the format above and imported.

### Custom auxiliary code schemes

An imported code table only knows the words written in it. Many auxiliary code tables only cover single characters; to let words be filtered too, make it into a scheme:

```
schemes/
  shouyou/
    scheme.toml
    table.tsv
```

```toml
type = "aux"           # Required
name = "My Shouyou"
table = "table.tsv"    # Optional, defaults to table.tsv; can also be .dict.yaml or an imported .qj

[words]
2 = "AaBa"             # Two-character words: the first code letter of each character
3 = "AaBaCa"
4 = "AaBaCaZa"         # Four characters or more: the first code letter of the first three characters and the last one
```

- `[words]` holds word code rules: when a word isn't in the code table, a rule is chosen by word length and the word's code is put together from each character's code; the longest rule also covers longer words. Words written in the code table still use their own codes.
- In a rule, capital letters pick characters: `A` is the first character, `B` the second… `Z` the last and `Y` the second to last; lowercase letters pick a letter within that character's code, `a` the first and `z` the last. This matches Rime's notation.
- When a character has several codes, words are computed for every combination, so typing any of them filters to the word.
- Without `[words]`, it behaves like an imported code table and only knows the words written in it.

Schemes are listed together with imported code tables in the 码表 (Code tables) group and only take effect once you choose them under 辅码方案 (Auxiliary code scheme); delete the folder when you no longer need it. If you change a scheme's files, the input method rereads them automatically within a second or two.
