---
title: Auxiliary code
order: 7
description: "Adding a few letters after the pinyin to narrow the candidates to the word you want: turning it on, how to use it, the trigger key, importing code tables, and showing codes on candidates."
---

Paging through candidates to find a word is tedious. Auxiliary code lets you add a few letters after the pinyin to narrow the candidates straight down to the word you want. Supported on macOS and Windows.

## Turning it on

Turn on 启用 (Enable) under 设置 → 输入 → 辅码 (Settings → Input → Auxiliary code), and choose a code table under 辅码方案 (Auxiliary code scheme). If you've never chosen one, the bundled stroke code is used. Only one table is used at a time: mixing several would give the same letter different meanings and the filtering would be less clean.
If it's on but no code table is available, the trigger key does nothing and `;` keeps its usual behaviour.

## How to use it

Type pinyin as usual, then press the trigger key (`;` by default) to enter auxiliary code: the candidates don't change yet, and a `;` appears at the end of the pinyin line.
Then type the code; with each letter the candidates narrow immediately, leaving only words whose code starts with the letters you typed.

For example, if 开发's code in the table is `kf`, typing `kaifa`, then `;`, then `kf` leaves only 开发.

- The code only filters words; it doesn't change how pinyin is typed and doesn't affect sentences.
- Words not in the code table aren't shown while in auxiliary code.
- You don't need to type the full code: once the candidates are right, press Space or a number to commit.
- `Backspace` deletes the last letter of the code; once the code is empty you're still in auxiliary code with all candidates back, and one more press leaves it (with 码删空后留在辅码 (Stay in auxiliary code when empty) off, it leaves as soon as the code is empty). `Esc` clears the code at once.
- With Shift + 字母 (Shift + letter) set to 进组句 (Join the input), typing a capital letter in auxiliary code is the same as pressing `Esc` first: the code is cleared, you're back to pinyin, and the letter joins the pinyin as usual.
- Words picked in auxiliary code are remembered too, so next time they rank higher with plain pinyin.

## Trigger key

`;` by default; change it under 设置 → 输入 → 辅码 (Settings → Input → Auxiliary code) to any single visible character such as `/` or a backtick.

Microsoft, Sogou and Ziguang double pinyin use `;` for the final ing: when an initial is left on its own at the end of the pinyin, `;` acts as pinyin (`x;` gives xing); only otherwise is it the trigger key.

## Code tables

The bundled stroke code: horizontal `h`, vertical `s`, left-falling `p`, dot and right-falling `n`, turning `z`. A single character uses its first four strokes plus its last stroke; a word uses the first stroke of each character, and words of four or more characters use the first three characters and the last one. Words you've created and words in imported dictionaries get codes by the same rule.

Other code tables have to be obtained yourself and imported: 设置 → 输入 → 码表 → 导入码表 (Settings → Input → Code tables → Import code table) accepts Rime code table files (`.dict.yaml`) and Qingjian `.qj` files.
When the import finishes, it shows how many entries were read, had codes, had no codes and were skipped, and switches to the table you just imported; to switch back, choose it under 辅码方案 (Auxiliary code scheme). Imported tables are listed in the 码表 (Code tables) group and can be removed.

For how to write a code table file, and how to let a table that only covers single characters filter words too, see [Custom schemes and code tables](../advanced/custom-schemes.md#Auxiliary-code-tables).

## Showing codes on candidates

With 设置 → 输入 → 辅码 → 候选上显示码 (Settings → Input → Auxiliary code → Show codes on candidates) on, the code is shown in square brackets right after the candidate, for example `鹤[rbm] crane`.
You can see it without entering auxiliary code: while typing plain pinyin, each candidate carries its code so you can learn them as you go; in auxiliary code it shows the code that matches what you've typed.
