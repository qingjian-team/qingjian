---
title: Your first input
order: 2
description: "Switching between Chinese and English, the candidate window, picking words and paging, sentence input, and undoing a wrong pick."
---

## Switching between Chinese and English

- **macOS**: `Caps Lock` (the 中 / 英 key on the keyboard). Light off means Chinese, light on means English.
- **Windows**: tap `Shift` (press and release on its own, without another key). The taskbar shows 中 or 英 to match, and A while Caps Lock is on; clicking that icon also switches.
  On Windows, `Caps Lock` only controls upper and lower case.
- **Linux**: tap `Shift`.

You can change the switch key or turn English mode off entirely; see [Chinese and English](../input/english-mode.md).

## The candidate window

While you type pinyin, the candidate window appears below the cursor: the top line shows the pinyin you've typed (split into syllables automatically), the candidates are below it, and the page number is at the bottom right. By default it's vertical with 9 candidates per page; both can be changed in Settings, see [Appearance and themes](../personalize/appearance.md).
When a string of pinyin can be split more than one way, it's split according to the reading of the highlighted candidate: `ganga` shows as `gan'ga` when 尴尬 is highlighted and as `gang'a` when 刚啊 is highlighted.
The smaller, lighter column to the right of the candidates is the gloss; see [Glosses beside candidates](../learning/translation.md).

## Picking words and paging

| Action | Keys |
|---|---|
| Commit the highlighted candidate | `Space` |
| Commit candidate N | Number keys `1` to `9` |
| Commit the pinyin letters as typed | `Enter` |
| Page | `[` `]`, or `PageUp` / `PageDown` |
| Move the highlight | `↑` / `↓` |
| Delete a letter (if words are already picked in front, undo the last pick first) | `Backspace` |
| Clear | `Esc` |

You can also click a candidate to commit it. The full key table is in the [Key reference](keys.md).

## Sentence input

There's no mode to switch: type a whole sentence and then commit it. The first candidate for `jintiantianqihenhao` is 今天天气很好 ("the weather is nice today"). Abbreviated pinyin works too: `jttqhh`.
For sentences, abbreviations and what happens when you mistype a letter, see [Sentences, abbreviations and correction](../input/sentence.md).

## Chinese punctuation

In Chinese mode, punctuation keys produce full-width punctuation: `,` gives ，, `.` gives 。, and `{` `}` give 「」. A `.` after a digit stays half-width (`3.14`).
For switching between full-width and half-width, and what punctuation keys do while you're typing pinyin, see [Punctuation, symbols and emoji](../input/punctuation.md).

## Picked the wrong word

Delete it with `Backspace` after committing, then type the same pinyin again and pick another candidate; the earlier pick isn't remembered. Accidentally committing a wrong word with `Space` isn't remembered either.
Deleting the last few words and retyping from the start works too: if you delete 沃德书 and retype it as 我的书, the 沃德 pick is treated as if it never happened.

When you pick a stretch of pinyin in several steps, the words picked so far stay in the input box, in front of the remaining pinyin, and are committed together only when the whole stretch is done. `Enter` commits them together with the remaining pinyin as typed; `Esc` cancels everything.
If you notice an earlier pick was wrong, `Backspace` undoes the last pick and turns that part back into pinyin so you can pick again; once all picks are undone, `Backspace` deletes letters.
If you'd rather commit each pick right away, turn off 分次选词先不上屏 (Keep partial picks until done) under 设置 → 输入 (Settings → Input).

When you pick a stretch of pinyin in several steps (pick one character first, then commit the rest as a sentence) and type it the same way a second time, the combined word is saved as a user word and from then on comes first when you type that stretch.
For the other ways Qingjian learns your habits, see [Typing habits](../personalize/habits.md).
