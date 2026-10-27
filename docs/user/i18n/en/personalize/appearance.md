---
title: Appearance and themes
order: 1
description: "The candidate window's seven built-in themes and light/dark modes, vertical and horizontal layouts, expanding horizontal layout into several rows, pinyin position and tones, fonts and sizes, transition animations."
---

Change how the candidate window looks under 设置 → 外观 (Settings → Appearance); the top of the page shows a preview drawn with your current settings. A theme looks the same on macOS and Windows.
Linux uses Fcitx's own candidate panel, whose look is set in Fcitx; for Android see [Android: vocabulary book and settings](../platforms/android-settings.md#Appearance).

## Themes

| Theme | Highlighted candidate |
|---|---|
| 青简绿 (Qingjian green, default) | Light green background, text colour unchanged |
| 系统蓝 (System blue) | System blue background, white text |
| 微信绿 (WeChat green) | WeChat green background, white text; AI candidates in blue |
| 玻璃 (Glass) | Blue capsule, white text; the horizontal layout is a single capsule. On macOS the window is clear glass that shows what's behind it, and the text turns darker or lighter with the background; on Windows it has a near-white opaque background |
| 樱花 (Sakura) | Pink-purple glass background, pink highlight block, serif candidates, part of speech and gloss on two lines; a character lying on the top left, with a cherry-blossom charm and ribbons |
| 大肥鱼 (Big fat fish) | Blue-edged glass background with ripples, light-blue highlight block, gloss on its own line below the word; a whale maid lying on the top edge of the window, clinging to the right side in vertical layout |
| 初音未来 (Hatsune Miku) | Cyan chamfered border and equalizer bars, cyan highlight block, gloss on its own line below the word; Hatsune Miku lying on the top edge of the window, clinging to the right side in vertical layout |
| 波奇酱 (Bocchi) | Pink double border, cream background, an audio cable looped around the window, pink highlight block; Bocchi with her guitar lying on the top edge of the window, hiding on the left in vertical layout |

Click a theme card to switch. 青简绿, 系统蓝, 微信绿 and 玻璃 each have a light and a dark version; choose follow system, light or dark under 外观 (Appearance) on the same page. 樱花, 大肥鱼, 初音未来 and 波奇酱 are light only.
Themes only apply when 渲染引擎 (Rendering engine) is set to 青简渲染器 (Qingjian renderer, the default).

To make a theme of your own, see [Custom themes](../advanced/custom-themes.md); once you've added a finished theme, reopen Settings and it appears among the theme cards.

## Layout

- **Vertical or horizontal**: vertical shows one candidate per row; horizontal puts the candidates in one row, with only the highlighted candidate's gloss on a separate line below and the page number at the end of the row.
- **Candidates per page**: choose 1 to 9 under 设置 → 候选 (Settings → Candidates).
- **Pinyin position**: inline and in the window, inline only, or in the window only. "Inline" shows the pinyin you're typing in the app's input box; terminals and some apps don't display inline pinyin well, so choose "in the window only" for them.

### Expanding horizontal layout into several rows

On macOS and Windows, in horizontal layout, you can tick 横排时 ↑ / ↓ 展开成多行 (Expand to rows with ↑ / ↓ in horizontal layout) (unticked by default). When ticked:

- `↑` / `↓` expand the row into a grid of 6 rows (one row per page of candidates) and move to the previous or next row at the same time; pressing `↑` on the first row returns to the first candidate.
- `←` / `→` move between candidates (also when not expanded); after `←` reaches the first candidate, pressing it again moves the pinyin cursor, and once `→` brings the cursor back to the end it moves between candidates again.
- Number keys pick within the row the highlight is on; when the highlight moves past the top or bottom edge of the grid it scrolls by row, and paging keys turn a whole screen.
- Once expanded, the window keeps its size for the rest of that input; candidates longer than four characters are cut off with "…" in their cell. Type more pinyin or press `Esc` to collapse back to one row.

When unticked, keys in horizontal layout work as in vertical layout: `↑` / `↓` move the highlight one at a time and `←` / `→` move the pinyin cursor.

## Showing tones on the pinyin line

On macOS and Windows you can turn on 拼音行显示声调 (Show tones on the pinyin line) under 设置 → 候选 (Settings → Candidates): the pinyin at the top of the candidate window is marked with the tones of the highlighted candidate's reading (`ganga` with 尴尬 highlighted shows `gān'gà`), and abbreviated pinyin is replaced with the full reading.
For candidates whose reading doesn't match the input (fuzzy pinyin, corrections, AI suggestions), the pinyin you typed is shown; characters whose reading is uncertain aren't marked. The pinyin in the app's input box doesn't change.

## Text

- **渲染引擎 (Rendering engine)**: 青简渲染器 (Qingjian renderer, default) or system drawing. Themes, fonts, key hints and so on only apply with the Qingjian renderer.
- **Font**: the system font by default.
- **Rare characters on Windows**: when the system's extended SimSun fonts are installed, the candidate window uses them automatically for rare characters. There is no need to change your theme or font. Characters not covered by the installed fonts may still appear as missing glyphs.
- **Candidate size, gloss size**: the theme's sizes by default; 恢复主题字号 (Restore theme sizes) goes back to them, and they then follow the new theme when you switch themes.

## Motion

With 过渡动画 (Transition animation) off, the highlight jumps straight into place and decorations that keep moving in a theme stop too. Animations also don't play when the system's Reduce Motion is on.
