---
title: Vietnamese Telex
description: Switching to Vietnamese Simple Telex with a double Shift tap, mixing Vietnamese and English, and restoring the keys you typed.
---

# Vietnamese Telex

Under 设置 → 输入法切换 (Settings → Input method switching), set 连按两次 Shift 切换第三输入法 (Double-tap Shift for a third input method) to 越南语 (Vietnamese). Your usual switch key still toggles between Chinese and the foreign language you used last; tapping Shift twice within 400 ms toggles between English and Vietnamese. A double tap in Chinese goes straight to the other foreign language, without flashing through the one in between.

Vietnamese uses Simple Telex: `aw` → `ă`, `aa` → `â`, `dd` → `đ`, `ee` → `ê`, `oo` → `ô`, `ow` → `ơ`, `uw` → `ư`; `s`, `f`, `r`, `x` and `j` type the five tones, and `z` removes the tone. Tone marks are placed by the modern rules, for example `hoas` → `hoá`.

The whole passage stays as composing text in the text box: Space confirms the current word so you can go on to the next one, and punctuation stays in the passage too; only Enter commits everything at once and ends the round. Backspace first undoes the Telex keys you actually typed; when the current word is empty, it goes on deleting the confirmed text that hasn't been committed yet. Esc turns the current word back into the keys you typed.

The first item in the candidate window is the current Telex result. After it come words and phrases that match the whole passage not yet committed, and when that gives too few, completions of the current word. Picking a full phrase replaces the whole passage instead of appending the same beginning again. Each Vietnamese candidate shows one or two common Chinese glosses on its right. When the result differs from the keys you typed, the original keys come last. Space picks the first item; numbers or a click pick any other.

Right-click a candidate to promote it, demote it, undo a demotion, or have the candidate model regenerate its Chinese glosses. Vietnamese candidate rankings are kept apart from Chinese pinyin ones.

Candidates work with clicks, number keys, Tab and the arrow keys: ↑ / ↓ (and ← / →) move the highlight, and Tab picks the highlighted candidate.
The Chinese glosses to the right of a candidate take Qingjian's usual shortcuts as well: Command + number commits the first gloss, Shift + Option + number the second; clicking a gloss does the same.

Mixed Vietnamese and English is judged by Vietnamese syllable structure, so words that clearly aren't Vietnamese syllables, such as `linux`, `changes` and `windows`, stay English. A few English words happen to be valid Telex too, for example `this` → `thí` and `mix` → `mĩ`; press Esc to get the original back.
