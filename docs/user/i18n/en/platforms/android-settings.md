---
title: "Android: vocabulary book and settings"
order: 5
description: "Qingjian for Android: the vocabulary book and statistics, dictionaries, appearance, keyboard settings, and exporting and clearing learning data."
---

All of the following are in the 青简 (Qingjian) app and take effect immediately on the open keyboard.

## Vocabulary book and statistics

- The 生词本 (Vocabulary book) page lists glosses in your learning language that you've seen beside candidates while typing, most recently seen first. The language at the top right changes the learning language directly.
- The search box finds words by the word itself, the matching Chinese or the example sentence, case-insensitively, and can be combined with filters.
- Filter by 生词 / 近 7 天新见 / 认识了 (New / First seen in the last 7 days / Known) (the last 7 days include today and roll; they don't reset on Monday or Sunday); filter English by A1–C2 and Japanese by N5–N1.
- Tap a word to see its Chinese, example sentence and counts; you can mark it 认识了 (Known) (no longer marked as new among candidates) or delete it from the vocabulary book.
- The top of the home screen shows typing statistics: characters today (with today's speed once you've typed for half a minute), the last 7 days and in total, characters per day for the last 30 days, plus first-choice rate, right first time, keystrokes per character and the number of days you've typed. With no input it shows 0 or "—".
- Tap any statistics card on the home screen (or the total card on the 我的 (Me) page) to open the statistics details: characters today, in the last 7 days, this month and in total; bar charts for the past 24 hours and the last 30 days; your busiest day, speed, first-choice rate, right first time and average keystrokes; and progress through each vocabulary level of your learning language.

## Dictionaries

我的 → 词库 (Me → Dictionaries) (or 键盘设置 → 输入 → 词库 (Keyboard settings → Input → Dictionaries)):

- Topic dictionaries: law, medicine, place names, idioms, famous lines of poetry, IT and computing, finance, food, animals, cars, historical figures. They ship with the app and can be turned on one by one. Idioms are on by default.
- Importing dictionaries: tap 导入词库 (Import dictionary) and choose files in the system file picker; you can select several. The format is the same as pinyin dictionaries on desktop, see [Dictionaries](../personalize/dictionaries.md#Importing-your-own-dictionaries). The name and entry count are shown after importing.
- Imported dictionaries can be turned on or off individually; after 删除 (Delete) they need to be imported again. Turning on, importing and deleting take effect immediately on the keyboard in use.

## Appearance

我的 → 外观 (Me → Appearance):

- Themes: the same seven themes as on desktop (青简绿, 系统蓝, 微信绿, 樱花, 大肥鱼, 初音未来, 波奇酱), each card showing what the candidate bar and keyboard look like. Once selected, the keyboard, candidate bar, symbol and emoji panels and quick translation bar all take on that theme's colours.
- Light/dark: follow system, light or dark. 樱花, 大肥鱼, 初音未来 and 波奇酱 are light only and stay light when dark is chosen.
- Candidate size: small, standard, large or extra large; candidates and glosses scale together, including when all candidates are expanded, and the candidate bar gets taller with larger sizes. The labels on the keys don't change.

The preview at the top of the page shows the current theme, light/dark mode and size.

## Keyboard settings

我的 → 键盘设置 (Me → Keyboard settings):

- **中文键盘 (Chinese keyboard)**: choose pinyin 26-key, 9-key, double pinyin, Wubi or Zhuyin; double pinyin has a choice of schemes, and Wubi can be mixed with pinyin.
- **学习 (Learning)**:
  - Learning language: changing it changes the glosses on the keyboard immediately; choosing 不显示 (Don't show) hides glosses and stops recording new words.
  - Learn typing habits: when off, words you pick often and words you create are no longer remembered; what's already learned still affects ranking.
- **输入 (Input)**:
  - Fuzzy pinyin: z = zh, c = ch, s = sh, n = l, f = h, l = r, an = ang, en = eng and in = ing are available; see [Pinyin schemes and fuzzy pinyin](../input/fuzzy-and-shuangpin.md#Fuzzy-pinyin). Not yet supported on the 9-key keypad.
  - Full-width punctuation for Chinese: when off, comma, full stop, question mark and so on are half-width on the Chinese keyboard. A point after a digit is always half-width.
  - Keep partial picks until done: on by default; when off, each pick is committed immediately.
  - English words while typing pinyin: input such as `hello` or `compa` brings English words among the candidates; you can also choose 中文排在英文词前面 (Chinese before English words).
  - Candidates on the English keyboard: when off, letters on the English keyboard are committed directly without candidates.
  - Traditional character forms: the forms used when 繁体 (Traditional) is on in the function panel — Taiwan, Hong Kong or standard; see [Traditional Chinese output](../input/traditional.md).
  - Dictionaries: opens the dictionaries page, see above.
- **剪贴板 (Clipboard)**: see [Android: tools and AI](android-tools.md#Clipboard-and-saved-phrases).
- **云端大模型 (Cloud LLM)**: see [Android: tools and AI](android-tools.md#Setting-up-AI-services).
- **尺寸 (Size)**: keyboard height, which you can also drag directly on the keyboard with ↕ in the toolbar; with full-screen adaptation on, a key is placed on each side of the gesture bar.
- **按键 (Keys)**: key vibration, key popups, keep Chinese when switching languages, hold space to talk, swipe typing; see [Android: keyboard and typing](android.md).

## Exporting and clearing

我的 → 高级 (Me → Advanced):

- Export data: saves learned words, the vocabulary book, typing statistics and saved phrases as a zip file; choose where to save it in the system's save screen. Clipboard history and imported dictionaries aren't included.
- Clear learning data: choose to clear typing habits (words you pick often, words you created, English words, your personal word order), the vocabulary book (all languages), or both; it takes effect immediately after confirming and can't be undone. Typing statistics, saved phrases, the clipboard, dictionaries and settings aren't affected.
