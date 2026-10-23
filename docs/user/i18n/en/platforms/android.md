---
title: "Android: keyboard and typing"
order: 3
description: "Qingjian for Android (preview): enabling it, keyboard types, candidates and glosses, dates and calculations, voice input and swipe typing."
---

Qingjian for Android is a preview with fewer features than the desktop versions, and it has no local sentence model yet, so its first choice for long sentences is weaker. It ships as an arm64 APK for Android 8.0 and later: download `qingjian-<version>-android-arm64.apk` from [GitHub Releases](https://github.com/qingjian-team/qingjian/releases/latest) and open it on the phone to install; when the system asks, allow your browser or file manager to install unknown apps. For a new version, download the new APK and install it over the old one; your typing habits and settings are kept. To build it yourself, follow the instructions in the `apps/android` folder of the source repository.
Pinyin conversion, dictionaries, learning and statistics all happen on the phone; only translation, gloss filling, cloud suggestions and voice input go online, and they require a provider you enter yourself.

The other two pages in this group: [Android: tools and AI](android-tools.md), [Android: vocabulary book and settings](android-settings.md).

## Enabling it

Open the 青简 (Qingjian) app; the card at the top of the home screen gives two steps: enable Qingjian in the system settings, then switch to Qingjian as the current input method.
The 试一试 (Try it) box on the home screen lets you try typing right away.

## Keyboard

- Tap the keyboard icon in the toolbar to choose an input method: pinyin 9-key, pinyin 26-key, double pinyin, Wubi (86 edition), Zhuyin (Dachen), English 26-key, Japanese romaji (beta).
- Pressing the 中英 key switches between Chinese and another language, and the small text in the key's corner is the language it will switch to; long-press it to choose another language.
- On the 9-key keypad, the column on the left lists the pinyin that share the keys while you type; tap one to fix that syllable. When you're not typing, it holds common punctuation.
- 123 opens the number keyboard; 符 on the Chinese keyboard opens categorized symbols, and !?# on the English keyboard opens a page of common symbols.
- Long-press the space bar and slide left or right to move the cursor; the Enter key shows Send, Search, Go and so on depending on the input field.
- In password, URL and email fields, keys go straight in without pinyin input, and nothing is learned.
- In input fields an app marks as private (such as a browser's incognito tab) you can type as usual, but typing habits aren't learned, new words aren't recorded, and translation, gloss filling and cloud suggestions send nothing.

## Candidates and glosses

- Each candidate has a gloss in your learning language below it, orange for new words; see [Glosses beside candidates](../learning/translation.md). The learning language can be English, Japanese, Spanish, Vietnamese, German, or none.
- Long-press a candidate to pop up its glosses; slide your finger onto a gloss and release to commit it; slide out of the bubble to cancel.
- Candidates that can be deleted (words you've created, words learned, English words) have an extra 删除 (Delete) at the end of the bubble: it deletes a word you created, or clears the learning records for that word. For a candidate with no gloss, the bubble only has 删除, and you need to slide up into the bubble to select it.
- The ⌄ at the right end of the candidate bar expands all candidates.
- When you pick a stretch of pinyin in several steps, the words picked first stay in the input box and the whole stretch is committed together once done; if you pick wrong, the delete key undoes the last pick so you can pick again. Enter commits them together with the remaining pinyin as typed. If you don't want this, turn off 分次选词先不上屏 (Keep partial picks until done) in the keyboard settings.

## Dates and calculations

- Type `rq`, `sj` or `xq` for today's date, the current time or the day of the week; when you type `riqi`, `shijian` or `xingqi`, the same forms follow right after 日期, 时间 or 星期.
- On the 9-key keypad, digits like `77` aren't taken as `rq`; tap r and then q in the pinyin column on the left to get the date. `7474` (riqi) also brings the date forms when it gives 日期.
- On the full keyboard, type `v`, tap 123, then type a calculation such as `v1+2`; the candidates give `3` and `1+2=3`. `v123` gives Chinese numerals and the amount forms.
- After typing `3+4` directly on the number keyboard and committing it, the candidate bar offers `7` and `3+4=7`: tap `7` to replace the expression with the result, or `3+4=7` to append `=7`. After typing `3×4=`, the candidates offer `12`. Dates (`2026-10-19`) and phone numbers aren't treated as calculations.

## Voice input

First choose a recognition service under 我的 → 语音识别 (Me → Speech recognition) in the Qingjian app. Currently you can choose Volcano Engine's Doubao streaming speech recognition: enable it in the Volcano Engine console, create an API key and enter it here, choose the billing type you signed up for (hourly or concurrent), and tap 测试连接 (Test connection) to check.
These settings are separate from the 云端大模型 (Cloud LLM) used for translation, and don't affect each other; the key is stored encrypted on the phone. Without a recognition service, the voice entry points only tell you it isn't available yet.

- Three entry points: hold the space bar for a moment (键盘设置 → 按键 → 长按空格说话 (Keyboard settings → Keys → Hold space to talk), on by default), the microphone in the toolbar, and the voice key in the bottom bar on full-screen phones. The first time, the system asks whether to allow Qingjian to use the microphone.
- As you speak, the recognized text appears in the input field (underlined, not final yet) and is finalized when you finish.
- Holding the space bar: release to finish; slide onto 撤回输入 (Withdraw) on the left and release to discard that part; slide onto the area on the right and release to finalize and immediately perform the field's action (Send in a chat box). Sliding left and right while holding the space bar still moves the cursor.
- Tapping in: tap the microphone or keyboard icon below when you've finished to return to the keyboard; tap ⌄ to finish and close the keyboard; the two arcs can be tapped too.
- While you speak, the audio is sent to Volcano Engine for recognition, which uses data and incurs service charges. Recognized text counts toward your typing statistics like typed text, and is converted to Traditional Chinese when Traditional output is on.

## Swipe typing

With 键盘设置 → 按键 → 滑行输入 (Keyboard settings → Keys → Swipe typing) on (off by default), on the full pinyin keyboard and the English keyboard you can slide your finger across a word's letters without lifting it, and when you release, the word that best matches the shape is recognized: the pinyin keyboard recognizes pinyin (swipe nihao for 你好; a single syllable works too), and the English keyboard recognizes English words.
Letters you pass on a straight line and doubled letters (the ll in hello) don't need special detours. The recognized letters bring up candidates as if tapped; when the English keyboard has no candidates, the word is committed directly (capitalized if Shift is on).

- Only pinyin and English words in the dictionary are recognized; type new words outside it by tapping.
- Words with nearly the same shape (to and too, pinyin z and zh) are chosen by how common they are; if it's wrong, backspace and try again or tap to correct it.
- The first swipe after turning it on prepares the word list first and is a little slower.
- The 9-key keypad, double pinyin, Wubi, Zhuyin and Japanese don't support swipe typing.

## Not yet supported

Offline on-device speech recognition, custom themes.
