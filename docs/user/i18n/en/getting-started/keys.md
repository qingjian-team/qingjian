---
title: Key reference
order: 3
description: "Qingjian's default keys on macOS, Windows and Linux: switching, picking, paging, moving the cursor, glosses, deleting candidates, the symbol panel, AI features and prefix keys."
---

This page only lists keys. In the tables, `⌥` is Option, `⌘` is Command, `⌃` is Control and `⇧` is Shift; "—" means the platform has no such key.
For keys you can change, see [Changing keys](../personalize/keys-settings.md); Android uses an on-screen keyboard, see [Android](../platforms/android.md).

## Switching

| Action | macOS | Windows | Linux |
|---|---|---|---|
| Switch Chinese / English | `Caps Lock` | Tap `Shift`; the system's `Ctrl + Space`; click 中 / 英 on the taskbar or the floating status bar | Tap `Shift` |
| Type uppercase English directly | Hold `⇧` in English mode | `Caps Lock` on (in either mode) | `Caps Lock` on |
| Type one capital letter while typing pinyin | `⇧ + letter` | `Shift + letter` | `Shift + letter` |
| Switch full-width / half-width punctuation | `⌃ + .` can be enabled (off by default) | Click the ，。 cell on the floating status bar; `Ctrl + .` can be enabled (off by default) | — |
| Switch to another input method | `⌃ + Space` (system) | `Win + Space` (system) | Fcitx's switch hotkey |

On Windows and Linux, the Chinese / English switch can also be a tap of `Ctrl` or `Ctrl + Alt + Space`; see [Chinese and English](../input/english-mode.md#Changing-or-turning-off-the-Chinese-English-switch).

## While typing pinyin

| Action | macOS | Windows | Linux |
|---|---|---|---|
| Commit the highlighted candidate | `Space` | `Space` | `Space` |
| Commit candidate N on this page | `1` to `9` | `1` to `9` | `1` to `9` |
| Commit with the mouse | Click a candidate; click the gloss to its right to commit that gloss | Same | Click a candidate |
| Commit the letters as typed | `Enter` | `Enter` | `Enter` |
| Delete the letter before the cursor (when picking in steps, undo the last pick first) | `Backspace` | `Backspace` | `Backspace` |
| Delete the letter after the cursor | `Delete` | — | `Delete` |
| Delete the syllable before the cursor | `⌥ + Backspace` | — | — |
| Delete all pinyin before the cursor | `⌘ + Backspace` | — | — |
| Clear | `Esc` | `Esc` | `Esc` |
| Move the highlight (pages automatically at the edge) | `↑` / `↓` | `↑` / `↓` | `↑` / `↓` |
| Page | `[` `]`, `PageUp` / `PageDown`, `⇧ + Tab` (previous page) | `[` `]`, `PageUp` / `PageDown`, `Shift + Tab` (previous page) | Same as Windows |
| Accept the AI sentence completion (next page if there is none) | `Tab` | `Tab` | `Tab` |
| Move the pinyin cursor | `←` / `→` | `←` / `→` | `←` / `→` |
| Move the cursor by syllable | `⌥ + ←` / `⌥ + →` | — | — |
| Move the cursor to the start / end | `⌘ + ←` / `⌘ + →`, or `Home` / `End` | `Home` / `End` | `Home` / `End` |
| Open the symbol panel | `↑` while the first candidate is highlighted | Same | — |
| Enter auxiliary code | `;` | `;` | — |
| Expand to several rows in horizontal layout (enable in Settings) | `↑` / `↓` | `↑` / `↓` | — |

When the cursor is in the middle of the pinyin, candidates are based only on the pinyin before the cursor (`ni|hao` gives 你); after committing, the rest of the pinyin stays and the cursor moves to the end.
A few keys differ with Dachen Zhuyin and Wubi; see [Pinyin schemes and fuzzy pinyin](../input/fuzzy-and-shuangpin.md#Dachen-Zhuyin) and [Wubi and mixed input](../input/wubi.md).

## In the symbol panel

| Action | Keys |
|---|---|
| Move | `↑` `↓` `←` `→` |
| Switch between the Search / Symbols / Emoji groups | `Tab` / `⇧ + Tab` |
| Switch category | `[` `]` |
| Scroll a screen | `PageUp` / `PageDown` |
| Commit | `Space`, `Enter`, a number key or double-click |
| Back to candidates | `Esc`, `Backspace` or the ✕ at the top right |

Typing a letter closes the panel and continues typing. Details in [Punctuation, symbols and emoji](../input/punctuation.md#Symbol-panel).

## In English mode

| Action | macOS / Windows / Linux |
|---|---|
| Pick the highlighted candidate | `Space`, `Tab` |
| Pick candidate N (if this page has no candidate N, the digit is appended to what you typed, e.g. `gpt6`) | Number keys |
| Move the highlight / page | `↑` / `↓`, paging keys |
| Output the letters as typed | `Enter`, punctuation |

## Glosses and deleting candidates

| Action | macOS | Windows | Linux |
|---|---|---|---|
| Commit the first gloss of candidate N | `⌥ + number` | `Ctrl + number` | `Alt + number` |
| Commit the second gloss (when the entry has one) | `⇧⌥ + number` | `Ctrl + Shift + number` | `Alt + Shift + number` |
| Commit the first gloss of the highlighted candidate | `⌃ + Space` | — | — |
| Delete candidate N | `⇧ + number` | `Shift + number` | `Shift + number` |

These only work while typing pinyin. See [Glosses beside candidates](../learning/translation.md#Committing-the-gloss-directly) and [Typing habits](../personalize/habits.md#Deleting-candidates-you-don't-want).

## AI services

All of these require AI services to be turned on first; see [AI services](../cloud/index.md).

| Action | macOS | Windows | Linux |
|---|---|---|---|
| Translate the text selected in an app | `⌃⌥T` | `Ctrl + Alt + T` | `Alt + Shift + T` |
| Translation result: replace the selection | `Enter` / `Space` / `1` | Same | Same |
| Translation result: keep the original | `Esc` | `Esc` | `Esc` |
| Ask for AI candidates manually while typing pinyin | `⌃⌥J` | `Ctrl + Alt + J` | — |
| Ask about a character | `u`, then type pinyin | Same | Same |

## Prefixes

| Input | Result |
|---|---|
| `rq` / `sj` / `xq` | Date / time / day of the week |
| `v` + expression, number or date | Calculation result, Chinese numerals and amounts, Chinese date |
| `u` + hexadecimal | The character with that code point |
| `u` + pinyin | Ask about a character (requires AI services) |

These are the same on all three platforms; details in [Dates, calculations and special characters](../input/shortcuts.md). With double pinyin and mixed input, `v` and `u` become `Shift + V` and `Shift + U`.
