---
title: Wubi and mixed input
order: 6
description: "Typing with shape-based codes such as Wubi (86 edition): turning it on, short codes and automatic commit, commit-on-next-code, the radical chart, and mixed input with Wubi and pinyin on together."
---

Shape-based input is supported on macOS and Windows (Android has its own Wubi keyboard); not yet on Linux.

## Turning it on

Under 设置 → 输入 → 形码 (Settings → Input → Shape-based), choose 五笔（86） (Wubi 86). Only Wubi 86 is bundled; you can also add your own shape-based scheme, see [Custom schemes and code tables](../advanced/custom-schemes.md#Custom-shape-based-schemes).

- **Wubi only**: set 拼音 (Pinyin) on the same page to 关 (Off).
- **Wubi–pinyin mixed input**: leave the pinyin scheme on; see [Mixed input](#Mixed-input) below.

## Typing

You type codes, not pronunciations: `ggll` gives 一, `khlg` gives 中国, `gant` gives 开发. The top of the candidate window shows the code exactly as typed.

- Words are looked up by however much of the code you've typed. Words whose code is exactly what you've typed come before words with longer codes; these are the level 1 / 2 / 3 short codes.
- Space commits the highlighted candidate and number keys pick; paging, deleting and committing work as with pinyin, and the left and right arrows move the cursor one letter at a time. Glosses beside candidates, typing habits and the vocabulary book work as usual.
- Features that belong to pinyin aren't available with Wubi: sentence input, abbreviated pinyin, fuzzy pinyin, spelling correction, and the [calculations and special characters](shortcuts.md) that start with `v` / `u`.

With Wubi only, a few more rules apply:

- **Auto-commit unique full codes**: when you've typed all 4 code letters and they match only one word, it's committed without pressing Space. For example, `trwu` commits 我们 directly; `khlg` matches 中国, 跑车 and 跨国, so it waits for you to pick.
- **Commit on next code**: if you type another letter when the code is already full, the highlighted candidate is committed first and that letter becomes the first code of the next word. For example, typing `trwu` right after `khlg` commits 中国 and then 我们.
- **Punctuation commits**: typing punctuation such as a comma or full stop commits the highlighted candidate first, followed by the full-width punctuation.
- **Clear empty codes**: when a code matches no word, Space or punctuation just clears the code instead of putting the letters into your text; press `Enter` if you want the letters.

The first two can be turned off: when Pinyin is 关 (Off), 设置 → 输入 (Settings → Input) shows two switches under 形码 (Shape-based), 满码唯一自动上屏 (Auto-commit unique full codes) and 顶字 (Commit on next code), remembered separately for each shape-based scheme.

### Radical chart

With 五笔（86） (Wubi 86) selected as the shape-based scheme, 设置 → 输入 (Settings → Input) also has 键位提示 (Key hints): the Wubi 86 radical chart appears next to the candidate window, with all the radicals of each key on it and the key name and zone number on the bottom row.
When it appears, its size and its opacity are the same settings as the double pinyin key hints; see [Key hints](fuzzy-and-shuangpin.md#Key-hints). Shape-based schemes you add yourself have no radical chart.

## Mixed input

With a pinyin scheme and a shape-based scheme both on, both codes and pinyin are accepted, so characters you can't type in Wubi can simply be typed in pinyin:

```text
gant     → 开发          (code)
nihao    → 你好          (no code match, pinyin used)
ggll     → 一            (not a syllable, code only)
woxiangxuehao → 我想学好  (from the 5th letter Wubi finds nothing, pinyin only)
```

- **Wubi words whose full code you've typed come first, then pinyin candidates, then Wubi words whose code you've only partly typed**: for `ga` the first candidate is 开; for `kai` it's still 开 from pinyin.
- Wubi codes are at most 4 letters, so **from the 5th letter on only pinyin is left**, with no need to switch by hand.
- A word matched by both appears only once.
- English words at the end of a sentence only work on the pinyin side; input longer than 4 letters can only be pinyin, and spelling correction works as usual.
- Calculations and asking about characters start with `Shift + V` and `Shift + U`; see [Dates, calculations and special characters](shortcuts.md#Changing-the-prefix-letters).
- Auto-commit of full codes, commit on next code and clearing empty codes are all inactive in mixed input: letters after a full code may be the start of more pinyin.

It also works the other way round, using mostly pinyin and Wubi only occasionally, except that level 1 / 2 short codes (`wo` is 伙 in Wubi) are ranked above the first pinyin candidate; if you don't want that, turn the shape-based scheme off.

## Adding your own Wubi words

You don't need to replace the whole code table: write the words to a file and import it from 设置 → 词库 → 导入词库 (Settings → Dictionaries → Import dictionary); see [Dictionaries](../personalize/dictionaries.md#Importing-Wubi-dictionaries).
