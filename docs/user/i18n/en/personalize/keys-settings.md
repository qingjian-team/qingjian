---
title: Changing keys
order: 2
description: "The 快捷键 (Shortcuts) page in Settings: paging keys, extra paging keys, highlight keys, the full-width/half-width punctuation shortcut, prefix letters for calculations and asking about characters, modifiers for glosses and deleting candidates, shortcuts for translation and AI suggestions."
---

All changeable keys are in 设置 → 快捷键 (Settings → Shortcuts), and changes take effect immediately. 恢复默认 (Restore defaults) at the bottom of the page resets the keys on this page, and below it are the keys that are fixed while typing pinyin and can't be changed. The full table of default keys is in the [Key reference](../getting-started/keys.md).

## Paging keys

翻页键 (Paging keys) can be `[` `]` (default), `,` `.` or `-` `=`: the first goes to the previous page, the second to the next. `PageUp` / `PageDown` always work.

- With `,` `.`, comma and full stop page while you're typing pinyin, instead of committing the candidate and adding punctuation.
- With `-` `=`, **the two keys type normally when there's nowhere to page to**: with no previous page while typing pinyin, `-` is just `-` (entering direct English input); with no next page, `=` is just `=`. The same goes for `-` among English candidates: on the first page, `-` is the hyphen in a word (`well-known`).
- `-` `=` mean those two keys themselves, not the `_` and `+` typed with Shift.
- With `[` `]` or `,` `.`, the keys still act as paging keys when there's nowhere to page to, and no punctuation is inserted into your text.

macOS and Windows also have 额外翻页键 (Extra paging keys): three sets you can combine, used together with the paging keys above. While typing pinyin these keys page first; operators in calculations, direct English input and the auxiliary code trigger key keep their usual roles.

## Highlight keys (macOS, Windows)

By default `↑` / `↓` move the highlight and `←` / `→` move the pinyin cursor. If you choose `←` / `→` instead, the highlight and the pinyin cursor form one line:
only when the pinyin cursor is at the end does `→` move to the next candidate, and only when the first candidate is highlighted does `←` go back into the pinyin. `↑` / `↓` then do nothing while typing pinyin (when you're not typing they go to the app as usual). This applies in both Chinese and English mode.

## Full-width/half-width punctuation shortcut (macOS, Windows)

With Ctrl + . 切换全半角标点 (Ctrl + . switches punctuation width) on, `Ctrl + .` (`⌃ + .` on macOS) switches punctuation between full-width and half-width for the current Chinese/English mode, without changing letters or digits and without switching Chinese/English.
It's off by default so it doesn't take over the app's own shortcut. On macOS, English-mode punctuation is always half-width, so the shortcut doesn't switch there and the key goes to the app.
To use a different key combination, see [Editing the configuration file](../advanced/config-file.md).

## Prefixes for calculations and asking about characters

表达式 (Expression) and 问字 (Ask about a character) are the prefix letters for calculations and asking about characters; only v, u and i are allowed, and picking a letter that's already in use swaps the two. See [Dates, calculations and special characters](../input/shortcuts.md#Changing-the-prefix-letters).

With ? 也进问字 (? also asks about a character) on, typing `?` when you're not typing pinyin, followed by pinyin, also asks about a character; see [Asking about a character](../cloud/question-and-translate.md#Asking-about-a-character).

## Glosses and deleting candidates

上屏第一个译词 (Commit first gloss), 上屏第二个译词 (Commit second gloss) and 删除候选 (Delete candidate) are each a modifier key used together with the number keys. Click one, then hold the modifier and press any number to record it.
上屏高亮候选的译词 (Commit the highlighted candidate's gloss, macOS) is a modifier key used together with `Space`: click it, then hold the modifier and press `Space` to record it.

Defaults:

| Action | macOS | Windows | Linux |
|---|---|---|---|
| Commit first gloss | `⌥` | `Ctrl` | `Alt` |
| Commit second gloss | `⇧⌥` | `Ctrl + Shift` | `Alt + Shift` |
| Commit the highlighted candidate's gloss | `⌃` | — | — |
| Delete candidate | `⇧` | `Shift` | `Shift` |

On macOS, avoid `⌃ + number` (the system uses it to switch desktops) and `⌘ + number` (apps use it to switch tabs); on Windows, apps take `Alt + number` as a menu shortcut, which is why `Ctrl` is the default. The three can't conflict with each other; if you record a key that can't be used, the reason is shown below that row.

## Translation and AI suggestions

- **翻译选中文字 (Translate selected text)**: click it, then press the key combination to record it. On Windows, after changing it, switch to another input method and back to Qingjian (or reopen the app) for it to take effect.
- **按需 AI 联想 (On-demand AI suggestions)** (macOS, Windows): the key combination for asking for AI candidates manually while typing pinyin; see [Cloud suggestions and sentence completion](../cloud/predict.md#On-demand-suggestions).

Press `Esc` or click elsewhere to cancel recording.
