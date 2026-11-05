---
title: "Android: keyboard and typing"
order: 3
description: "Qingjian for Android (preview): enabling it, keyboard types, candidates and glosses, dates and calculations, voice input, swipe typing and the game keyboard."
---

Qingjian for Android is a preview with fewer features than the desktop versions, and it has no local sentence model yet, so its first choice for long sentences is weaker. It ships as an arm64 APK for Android 8.0 and later: download `qingjian-<version>-android-arm64.apk` from [GitHub Releases](https://github.com/qingjian-team/qingjian/releases/latest) and open it on the phone to install; when the system asks, allow your browser or file manager to install unknown apps. For a new version, download the new APK and install it over the old one; your typing habits and settings are kept. To build it yourself, follow the instructions in the `apps/android` folder of the source repository.
Pinyin conversion, dictionaries, learning and statistics all happen on the phone; only translation, gloss filling, cloud suggestions and voice input go online, and they require a provider you enter yourself.

The other two pages in this group: [Android: tools and AI](android-tools.md), [Android: vocabulary book and settings](android-settings.md).

## Enabling it

Open the 青简 (Qingjian) app; the card at the top of the home screen gives two steps: enable Qingjian in the system settings, then switch to Qingjian as the current input method.
The 试一试 (Try it) box on the home screen lets you try typing right away.

## Keyboard

- The Qingjian mark at the left end of the toolbar opens the function panel, and the right end hides the keyboard. The row in between is yours to arrange: in the function panel, 常用功能 → 定制工具栏 (Common → Customize toolbar), the top row shows what's on the toolbar now; tap the "−" at the top right of a tool to remove it. Below are all the tools (emoji, switch keyboard, voice, symbols, clipboard, translate, keyboard height, saved phrases, switch input method, settings), with those already placed highlighted; tap one that isn't placed to add it to the end, or one that is to remove it, and the toolbar follows the order you added them. 恢复默认 (Restore defaults) at the top right goes back to the original layout. When you're done, tap ✓ at the top left to save and return to the keyboard; the system back button leaves without saving. With five or more tools the toolbar scrolls sideways.
- Tap the keyboard icon in the toolbar to choose an input method: pinyin 9-key, pinyin 26-key, double pinyin, Wubi (86 edition), Zhuyin (Dachen), English 26-key, Japanese romaji (beta).
- Pressing the 中英 key switches between Chinese and another language, and the small text in the key's corner is the language it will switch to; long-press it to choose another language.
- On the 9-key keypad, the column on the left lists the pinyin that share the keys while you type; tap one to fix that syllable. When you're not typing, it holds common punctuation, so the 9-key bottom row has no separate comma and period keys and the space bar is wider.
- Tap Shift once to capitalize only the next letter; tap it twice quickly to lock capitals, and once more to unlock. The Shift key is lit for a single capital, and lit with a small dot at its top right when locked.
- 123 opens the number keyboard; 符 on the Chinese keyboard opens categorized symbols, and !?# on the English keyboard opens a page of common symbols.
- Input fields for phone numbers, numbers, dates and times open the number keyboard directly; 返回 (Back) still switches to letters, and moving to another field brings back the keyboard you had.
- The digit or symbol in a key's corner: swipe up on the key to commit it (the 9-key keypad gives digits), with no need to long-press. Only the direction counts: up and to the left or right works, even past the key's edges, and you always get the key you pressed; sideways, downward or curving swipes don't count. Turn it off under 设置 → 键盘 → 手势 → 上滑输入 (Settings → Keyboard → Gestures → Swipe up for symbols); on by default.
- Long-press a letter for a bubble (capital, corner symbol, lowercase) and slide left or right to pick one before lifting; lifting without sliding commits the corner symbol. To get the capital on a plain long press, turn on 设置 → 键盘 → 手势 → 长按出大写 (Settings → Keyboard → Gestures → Long press for capitals); off by default.
- The categorized symbol panel has an even number of symbols per row, so paired brackets and quotes (【】《》“”) stay on the same row.
- While typing pinyin, tap the pinyin row above the candidates to put the cursor there; the letters you type and delete go in at that spot, so you can add a missed letter or fix a wrong one. With the cursor in the middle, candidates cover only the pinyin before it; after you pick one, carry on with the rest. With the cursor at the start, Delete removes from the end. When double pinyin shows full pinyin in that row, the cursor stops between syllables (the boundary nearest your tap); when it shows the keys you typed, it stops between keys.
- Hold the space bar and slide left or right to move the cursor; the farther you slide, the farther it moves, one character per step. It doesn't respond while you're typing pinyin: pick a word or press Enter first, or tap the pinyin row to move within the pinyin. Turn it off under 设置 → 键盘 → 手势 → 空格滑动移动光标 (Settings → Keyboard → Gestures → Slide space to move cursor); on by default.
- The Enter key shows Send, Search, Go and so on depending on the input field.
- English keyboard: the candidate bar offers English words, and you tap the one you want; space, digits, punctuation and Enter commit the letters as typed rather than a candidate, so verification codes, model numbers and similar letter strings can just be typed. You can tap Shift for capitals mid-word too.
- The small initials and finals on double pinyin keys can be turned off with 键上显示声韵母 (Show initials and finals on keys) under 设置 → 输入 (Settings → Input), shown when double pinyin is chosen.
- Microsoft double pinyin's `;` (ing) sits at the bottom left where Shift was: tap it for `;`, swipe up for Shift (swipe twice quickly to lock capitals; the `;` key lights up like Shift and gets a small dot when locked), and the second row has 9 keys as in full pinyin. Other schemes that use `;` (Sogou, Ziguang) still have an extra `;` key at the end of the second row.
- 设置 → 键盘 → 按键 → 布局 (Settings → Keyboard → Keys → Layout) switches from 默认 (Default) to 左手便捷 (Left-handed) (左手布局 (Left-handed layout) in the keyboard's function panel does the same): the symbol panel's categories run down the left column and 返回 (Back) is at the bottom left, leaving only delete and lock in the right column.
- In password, URL and email fields, keys go straight in without pinyin input, and nothing is learned.
- In input fields an app marks as private (such as a browser's incognito tab) you can type as usual, but typing habits aren't learned, new words aren't recorded, and translation, gloss filling and cloud suggestions send nothing.

## Candidates and glosses

- Each candidate has a gloss in your learning language below it, orange for new words; see [Glosses beside candidates](../learning/translation.md). The learning language can be English, Japanese, Spanish, Vietnamese, German, or none.
- Long-press a candidate to pop up its glosses; slide your finger onto a gloss and release to commit it; slide out of the bubble to cancel.
- Candidates that can be deleted (words you've created, words learned, English words) have an extra 删除 (Delete) at the end of the bubble: it deletes a word you created, or clears the learning records for that word. For a candidate with no gloss, the bubble only has 删除, and you need to slide up into the bubble to select it.
- The ⌄ at the right end of the candidate bar expands all candidates.
- When you pick a stretch of pinyin in several steps, the words picked first stay in the input box and the whole stretch is committed together once done; if you pick wrong, the delete key undoes the last pick so you can pick again. Enter commits them together with the remaining pinyin as typed. If you don't want this, turn off 分次选词先不上屏 (Keep partial picks until done) under 设置 → 输入 (Settings → Input).

## Dates and calculations

- Type `rq`, `sj` or `xq` for today's date, the current time or the day of the week; when you type `riqi`, `shijian` or `xingqi`, the same forms follow right after 日期, 时间 or 星期.
- The pinyin column on the left of the 9-key keypad lists every syllable and single letter the first few keys can spell, longest first: `643` gives nie, mie, ni, mi and m, n, o; tap ni and the rest is read after ni.
- Interjections can be picked from the column too: `64` → ng gives 嗯, `46` → hm gives 噷, `464` → hng gives 哼; a lone `6` picked as n or m also offers 嗯 or 呣.
- On the 9-key keypad, digits like `77` aren't taken as `rq`; tap r and then q in the pinyin column on the left to get the date. `7474` (riqi) also brings the date forms when it gives 日期.
- On the full keyboard, type `v`, tap 123, then type a calculation such as `v1+2`; the candidates give `3` and `1+2=3`. `v123` gives Chinese numerals and the amount forms.
- After typing `3+4` directly on the number keyboard and committing it, the candidate bar offers `7` and `3+4=7`: tap `7` to replace the expression with the result, or `3+4=7` to append `=7`. After typing `3×4=`, the candidates offer `12`. Dates (`2026-10-19`) and phone numbers aren't treated as calculations.

## Voice input

First choose a recognition service under 设置 → 语音识别 (Settings → Speech recognition) in the Qingjian app. Currently you can choose Volcano Engine's Doubao streaming speech recognition: enable it in the Volcano Engine console, create an API key and enter it here, choose the billing type you signed up for (hourly or concurrent), and tap 测试连接 (Test connection) to check.
These settings are separate from the 云端大模型 (Cloud LLM) used for translation, and don't affect each other; the key is stored encrypted on the phone. Without a recognition service, tapping a voice entry point shows a line in place of the toolbar; tap it to open the 语音识别 (Speech recognition) page.

- Three entry points: hold the space bar for a moment (设置 → 键盘 → 手势 → 长按空格说话 (Settings → Keyboard → Gestures → Hold space to talk), on by default), the microphone in the toolbar, and the voice key in the bottom bar on full-screen phones. The first time, the system asks whether to allow Qingjian to use the microphone.
- As you speak, the recognized text appears in the input field (underlined, not final yet) and is finalized when you finish.
- Holding the space bar: release to finish; slide onto 撤回输入 (Withdraw) on the left and release to discard that part; slide onto the area on the right and release to finalize and immediately perform the field's action (Send in a chat box). Sliding left and right while holding the space bar still moves the cursor.
- Tapping in: tap the microphone or keyboard icon below when you've finished to return to the keyboard; tap ⌄ to finish and close the keyboard; the two arcs can be tapped too.
- While you speak, the audio is sent to Volcano Engine for recognition, which uses data and incurs service charges. Recognized text counts toward your typing statistics like typed text, and is converted to Traditional Chinese when Traditional output is on.

## Swipe typing

With 设置 → 键盘 → 手势 → 滑行输入 (Settings → Keyboard → Gestures → Swipe typing) on (off by default), on the full pinyin keyboard and the English keyboard you can slide your finger across a word's letters without lifting it, and when you release, the word that best matches the shape is recognized: the pinyin keyboard recognizes pinyin (swipe nihao for 你好; a single syllable works too), and the English keyboard recognizes English words.
Letters you pass on a straight line and doubled letters (the ll in hello) don't need special detours. The recognized letters bring up candidates as if tapped; when the English keyboard has no candidates, the word is committed directly (first letter capitalized if Shift is lit, all capitals when locked).

- Only pinyin and English words in the dictionary are recognized; type new words outside it by tapping.
- Words with nearly the same shape (to and too, pinyin z and zh) are chosen by how common they are; if it's wrong, backspace and try again or tap to correct it.
- The first swipe after turning it on prepares the word list first and is a little slower.
- The 9-key keypad, double pinyin, Wubi, Zhuyin and Japanese don't support swipe typing.

## Game keyboard

When you play a game in landscape, the normal keyboard covers almost the whole screen. Once the game keyboard is on for an app, typing in that app in landscape brings up a small, semi-transparent floating keyboard instead, and you can keep tapping the game outside it; in portrait you still get the normal keyboard.

- Turning it on: while typing in that app in landscape, tap the Qingjian mark at the left end of the toolbar, then 游戏键盘 (Game keyboard) under 常用功能 (Common) in the function panel; the floating keyboard appears right away. Tap it again to turn it off. You can also tick the app under 设置 → 键盘 → 游戏键盘 (Settings → Keyboard → Game keyboard), which lists only apps you've typed in with Qingjian in landscape.
- Hold the bar at the top of the keyboard to drag it anywhere; the position is remembered for each app.
- The half-filled circle at the top left cycles the transparency through opaque, 20%, 40% and 60%; the keyboard button next to it switches back to the normal keyboard and turns the game keyboard off for this app; ⌄ on the right hides the keyboard.
- 设置 → 键盘 → 游戏键盘 (Settings → Keyboard → Game keyboard) also sets the transparency, the keyboard size (small / medium / large) and whether glosses appear beside candidates (off by default; while they're off, new words aren't recorded either). With the main switch off, ticked apps also get the normal keyboard.
- The floating keyboard is always dark; candidates, panels and the toolbar work as on the normal keyboard.

## Not yet supported

Offline on-device speech recognition, custom themes.
