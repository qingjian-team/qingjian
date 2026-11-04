---
title: Custom phrases
order: 3
description: "Typing a fixed phrase, address or long reply with a short input code, ordered by weight under the same code; phrases can be imported and exported; on macOS, the system's text replacements work too."
---

In 设置 → 自定义短语 (Settings → Custom phrases), give a piece of text you use often an input code; when you type that code, the text comes first among the candidates.

## Adding and editing

Each phrase is a row on the page: an on/off switch (labelled with the input code), its weight, the first line of its content, and 编辑 (Edit); the last two rows are 新增… (Add…) and 导入、导出 (Import, export).

Adding and editing happen in a card that pops up:

- **输入码 (Input code)**: 1–32 lowercase English letters, such as `dz`. No digits: while typing, digit keys pick candidates. To give a numbered symbol like ⑪ a code, spell it in letters, such as `yqsy`; circled numbers and circled ideographs (㊣) can also be found in the [symbol panel](../input/punctuation.md#Symbol-panel) by searching 序号 or 带圈汉字.
- **权重 (Weight)**: a whole number from 1 to 999, 10 for a new phrase. Under the same input code, heavier phrases come first; weights can be equal, and equal ones keep their order on the page.
- **内容 (Content)**: can span several lines and contain spaces, addresses, long replies or punctuation, and is committed exactly as written. The box wraps to its width; for very long content it grows to at most 12 lines and then shows the lines around the cursor.

Click 保存 (Save) to apply; `Esc` or 取消 (Cancel) discards the changes. When editing, the card has 删除 (Delete) on its left, which needs two clicks to confirm.

For example, add two phrases for `ee`: `：` with weight 20 and `；` with weight 10. Type `ee` and press Space to get ：, or press 2 to get ；.

In the candidate window, a multi-line phrase is shown on one line (line breaks drawn as `↵`), and a very long one shows only its first 30 or so Chinese-character widths followed by …; the full content is still what gets committed.

## Time variables

The content can contain time variables, replaced with the current time when the candidate appears, so what you see in the candidate window is what gets committed:

| Write | Becomes | Example |
|---|---|---|
| `{year}` | Year, four digits | 2026 |
| `{month}` `{day}` `{hour}` | Month, day, hour, no leading zero | 3, 8, 9 |
| `{minute}` `{second}` | Minute, second, two digits | 05, 07 |
| `{weekday}` | Day of the week, 一 to 日 | 日 |

For example, give `jt` the content 今天是{year}年{month}月{day}日，星期{weekday} and typing `jt` gives 今天是2026年3月8日，星期日; `{weekday}` is a single character, so you can put 星期 or 周 in front of it.
To output braces as they are, double them: `{{year}}` gives `{year}`. Anything not recognized (such as `{abc}`) stays as written. These variables are also replaced in macOS system text replacements.

## Importing and exporting

To move to another computer or keep a backup, click 导出… (Export…) in the 导入、导出 row to save all phrases (including switched-off ones) as a `.toml` file; on the other computer, click 导入… (Import…) and pick that file.

- Imported phrases are added after your existing ones, keeping the weights and switches from the file; a phrase whose content already exists under the same input code is not added again, so importing the same file twice is harmless.
- Plain text with one phrase per line as text, Tab, input code, Tab, weight (such as Rime's `custom_phrase.txt`) also works, for bringing phrases over from another input method; the weight is optional, and values outside 1–999 count as 1 or 999. This kind of text can't hold line breaks, so use the exported `.toml` for multi-line phrases.
- The configuration file `config.toml` can't be imported here: settings and phrases are imported and exported separately, settings on the 高级 (Advanced) page; see [Moving to another computer](../advanced/data-files.md#Moving-to-another-computer).
- Entries written wrongly in the file are skipped; afterwards the row says how many were added, how many already existed and how many were wrong, and points out the line of the first wrong one and what is wrong with it.

The exported file uses the same `[[custom_phrases]]` format as the configuration file, so you can also edit it in a text editor.

## Ordering rules

- All phrases for an input code come before the ordinary candidates, and ordinary pinyin candidates follow; neither your typing habits nor the local sentence model moves a phrase. AI candidates come after the phrases.
- If there are more phrases than fit on a page, they continue on the next page; you pick by the number on the current page.
- The same input code can't have two phrases with the same content; on a conflict, the card explains why and keeps your draft.
- Positions set before 0.3 become weights automatically: position 1 becomes 10, position 2 becomes 9, and so on, so the order stays the same.
- Choosing 置顶 (Pin to top) from a candidate's right-click menu while typing also adds an entry here, weighted above every existing phrase for that input code (999 at most); see [Candidate right-click menu](habits.md#Candidate-right-click-menu).

They also work in English mode, but only phrases whose content has no Chinese in it come through: once the whole code is typed, the phrase comes before the English word candidates (`dont` → `don't`). A phrase with Chinese content (including one added by right-clicking 置顶 (Pin to top) in Chinese mode) won't appear in English mode, so `Space` there can't commit Chinese. With Traditional Chinese output on they're still committed as written.
If a phrase is changed in another settings window while you're editing it, saving warns you and keeps your draft instead of overwriting.

## System text replacements (macOS)

With 也用系统的文本替换 (Also use system text replacements) on (the default), entries in System Settings → Keyboard → Text Replacements also work while you type pinyin:
once you've typed the whole shortcut, the phrase comes before the candidates (it counts as weight 10, and phrases from this page come first at equal weight), and `Space` or a number key commits it.
Only entries whose shortcut is all lowercase English letters work; entries added or changed in System Settings take effect the next time you switch to Qingjian. They aren't listed on this page; change them in System Settings.

Android's saved phrases are in the clipboard panel; see [Android: tools and AI](../platforms/android-tools.md#Clipboard-and-saved-phrases).
