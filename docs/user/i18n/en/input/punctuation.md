---
title: Punctuation, symbols and emoji
order: 3
description: "Chinese punctuation and switching between full-width and half-width, the two ways punctuation behaves while typing pinyin, the symbol panel, and the emoji, kaomoji and symbols that follow candidates."
---

## Chinese punctuation

When you're not typing pinyin, typing `, . ? ! : ; ( ) [ ] { } < > \ ^ _ $ ~` in Chinese mode gives full-width punctuation; `{` `}` give the corner brackets 「」, and `"` `'` alternate between opening and closing quotes. A `.` after a digit stays half-width (`3.14`).

### Full-width and half-width

- 中文标点 (Chinese punctuation) in 设置 → 中英文 (Settings → Chinese/English) can be set to half-width; letters and digits aren't affected.
- On Windows and Linux, English mode has its own 英文标点 (English punctuation), half-width by default; on macOS, English mode is always half-width.
- On Windows, clicking the ，。 cell on the floating status bar switches full-width / half-width for the current mode at any time.
- On macOS and Windows you can enable `Ctrl + .` (`⌃ + .` on macOS) under 设置 → 快捷键 (Settings → Shortcuts) to switch full-width / half-width for the current mode. It's off by default so it doesn't take over the app's own shortcut.

## Typing punctuation while typing pinyin

Typing punctuation halfway through some pinyin has one of two effects, set by 标点先上屏候选 (Punctuation commits the candidate first) in 设置 → 中英文 (Settings → Chinese/English):

- **Off (default)**: you enter **direct English input**: the pinyin and everything typed after it are shown as typed, and `Space` / `Enter` commit them as typed. In Chinese mode you can type strings with symbols such as `no-way` or `gpt-6` directly.
- **On**: punctuation that turns full-width (`,` `.` `?` `!` and so on, except paging keys) first commits the highlighted candidate and then adds the full-width punctuation: type `nihao,` in one go to get 你好，.

Either way:

- `'` is the syllable separator (`xi'an`).
- Symbols that don't turn full-width (`-` `/` `@`) always enter direct English input.
- Typing an email: after `qingjian@` the candidates list full addresses at common providers (qq.com, 163.com, gmail.com…); type `g` and only gmail.com is left. Move the highlight onto an address with the arrow keys and press `Space`, or click it, to enter that address without a trailing space; `Enter` still enters what you typed.
- To follow a candidate with half-width punctuation, switch to English mode first, or set Chinese punctuation to half-width.
- If you've chosen a pair of punctuation keys as paging keys (such as `,` `.`), those two keys page while you type pinyin; see [Changing keys](../personalize/keys-settings.md#Paging-keys).

## Symbol panel

macOS and Windows have a symbol panel. Press `↑` after typing (while the highlight is still on the first candidate and hasn't moved) to open it. When you're not typing: on macOS, choose 符号面板… (Symbol panel…) in the input method menu in the menu bar;
on Windows, click the 符 cell on the floating status bar, or right-click the 中 / 英 icon on the taskbar and choose 符号面板…. The panel appears next to the mouse (without the Search group in this case). It has three groups:

- **Search**: uses the pinyin you just typed as the search term: `ku` gives 😢 😭 (T_T), `jiantou` gives every arrow, `alpha` gives α. The group is absent when nothing is found.
- **Symbols**: common, Chinese and English punctuation, math, arrows, Greek letters, numbering, currency and units, fractions, superscripts and subscripts, decorative, geometric and box drawing, Hanyu Pinyin (with tones), Zhuyin, hiragana, katakana, IPA, Latin letters, vertical punctuation, full-width letters, mathematical letters.
- **Emoji**: common emoji, kaomoji, and all emoji grouped by category (smileys, people, animals and nature, food, travel and places, activities, objects, symbols, flags).

`Tab` switches group, `[` `]` switch category, arrow keys move, the mouse wheel or trackpad scrolls a row at a time, and `Space` / `Enter` or a double-click commits; the pinyin you just typed is cleared after committing. `Esc` or the ✕ at the top right returns to the candidates.
The bottom row shows the Chinese name of the highlighted item (😂 笑哭了, € 欧元符号). The panel currently uses the system style and doesn't follow the theme. Emoji that are too new can't be displayed on older systems, so the panel only goes up to Emoji 15.1.

## Emoji, kaomoji and symbols after candidates

When one of the first few candidates has a matching emoji, kaomoji or symbol, it follows right after that word, labelled with the word on its right:

| Input | Candidates |
|---|---|
| `ku` | 哭 😢 😭 (T_T) |
| `xiaosi` | 笑死 🤣 |
| `yuedengyu` | 约等于 ≈ |
| `sheshidu` / `erfenzhiyi` / `meiyuan` | 摄氏度 ℃, 二分之一 ½, 美元 $ |
| `alpha` (English candidate) | alpha α |
| `smile` in English mode | smile 🙂 😊 |

After the same word they come in the order emoji, kaomoji, symbols. Each has its own limit: under emoji, 颜文字 (Kaomoji) and 符号 (Symbols) in 设置 → 候选 (Settings → Candidates), choose not to show them or to show at most 1–3 (by default 3 emoji and 1 each of kaomoji and symbols).
Committing these candidates isn't recorded as a typing habit.
