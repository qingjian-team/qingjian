---
title: "Android: vocabulary book and settings"
order: 5
description: "Qingjian for Android: the home screen, vocabulary book and statistics, the Input, Keyboard, Appearance and Dictionaries pages under Settings, and exporting and clearing learning data."
---

All of the following are in the 青简 (Qingjian) app and take effect immediately on the open keyboard. The app has three tabs at the bottom: 首页 (Home), 生词本 (Vocabulary book) and 设置 (Settings).

## Home

- Until Qingjian is your current input method, the home screen shows two setup steps: enable Qingjian in the system settings, then switch to Qingjian. Each finished step gets a tick.
- Once it's set up, the top of the home screen shows characters today (with the number of English words), characters per day for the last 30 days, and characters in the last 7 days, first-choice rate and keystrokes per character; with no input it shows 0 or "—". Tap this card to open the statistics details.
- Below it are a few new words you've seen recently; tap 生词本 (Vocabulary book) to go to the vocabulary book. It's hidden when no learning language is chosen or there are no new words yet.
- At the bottom, 试一试 (Try it) lets you try typing right away.

## Vocabulary book and statistics

- The 生词本 (Vocabulary book) tab lists glosses in your learning language that you've seen beside candidates while typing, most recently seen first. The language at the right above the word list changes the learning language directly.
- The search box finds words by the word itself, the matching Chinese or the example sentence, case-insensitively, and can be combined with filters.
- Filter by 生词 / 近 7 天新见 / 认识了 (New / First seen in the last 7 days / Known) (the last 7 days include today and roll; they don't reset on Monday or Sunday); filter English by A1–C2 and Japanese by N5–N1.
- Tap a word to see its Chinese, example sentence and counts; you can mark it 认识了 (Known) (no longer marked as new among candidates) or delete it from the vocabulary book.
- Statistics details: switch between today, the last 7 days, this month and all time at the top to see the characters and bar chart for that period (by hour for today, by day otherwise); below are your busiest day, speed, first-choice rate, right first time and average keystrokes, plus progress through each vocabulary level of your learning language.

## Settings tab

The top of the 设置 (Settings) tab shows whether Qingjian is the current input method; tap 切换 (Switch) to open the system's input method picker. Below it, settings are grouped:

- **打字 (Typing)**: 输入 (Input), 键盘 (Keyboard), 外观 (Appearance) and 词库 (Dictionaries), each its own page, described below.
- **学语言 (Learning a language)**: the learning language; changing it changes the glosses on the keyboard immediately. Choosing 不显示 (Don't show) hides glosses and stops recording new words.
- **剪贴板 (Clipboard)**: the clipboard and local network sync; see [Android: tools and AI](android-tools.md#Clipboard-and-saved-phrases).
- **联网服务 (Online services)**: the cloud LLM and speech recognition, both using a provider you enter yourself; if you don't, Qingjian doesn't go online. For the cloud LLM see [Android: tools and AI](android-tools.md#Setting-up-AI-services); for speech recognition see [Android: keyboard and typing](android.md).
- **数据 (Data)** and **关于 (About)**, see the end of this page.

The right side of each row shows the current setting, such as the input method or theme in use.

## Input

设置 → 输入 (Settings → Input):

- **中文输入方案 (Chinese input method)**: pinyin 26-key, 9-key, double pinyin, Wubi or Zhuyin; tap a tile to switch. With double pinyin you can choose the scheme and turn off the small initials and finals on the keys; with Wubi you can mix in pinyin. English always uses the full keyboard.
- **拼音 (Pinyin)**:
  - Fuzzy pinyin: z = zh, c = ch, s = sh, n = l, f = h, l = r, an = ang, en = eng and in = ing are available; see [Pinyin schemes and fuzzy pinyin](../input/fuzzy-and-shuangpin.md#Fuzzy-pinyin). Not yet supported on the 9-key keypad.
  - Spelling correction: when you mistype pinyin, corrected candidates are offered; when what you typed still reads as valid pinyin, corrections only come later as alternatives. On by default; it's the same setting as 输入 → 纠错 (Input → Correction) on desktop.
  - Keep partial picks until done: on by default; when off, each pick is committed immediately.
  - 上屏后联想 (Suggest the next word): after you pick a word, the candidate bar offers words likely to follow it, committed with a tap; on by default.
- **中英混打 (Mixed Chinese and English)**:
  - English words while typing pinyin: input such as `hello` or `compa` brings English words among the candidates; you can also choose 中文排在英文词前面 (Chinese before English words).
  - Candidates on the English keyboard: when on, tap a candidate to get it, and space, digits and punctuation commit the letters as typed; when off, letters on the English keyboard are committed directly without candidates.
- **候选后面 (After candidates)**: three rows, emoji, 颜文字 (Kaomoji) and 符号 (Symbols), each set to 不显示 (Off) or at most 1–3 per word (`kaixin` → 开心 😄, `ku` → 哭 (T_T), `yuedengyu` → 约等于 ≈); by default 3 emoji and 1 each of kaomoji and symbols, the same as the items on the desktop Candidates page.
- **标点与字形 (Punctuation and character forms)**:
  - Full-width punctuation for Chinese: when off, comma, full stop, question mark and so on are half-width on the Chinese keyboard. A point after a digit is always half-width.
  - Auto-close paired punctuation: on the Chinese and Japanese keyboards, typing an opening bracket, quote or title mark adds the closing one with the cursor in between; on by default. 英文键盘也补全 (Also on the English keyboard) covers half-width brackets and double quotes and is off by default. See [Paired punctuation](../input/punctuation.md#Paired-punctuation).
  - Traditional character forms: the forms used when 繁体 (Traditional) is on in the function panel — Taiwan, Hong Kong or standard; see [Traditional Chinese output](../input/traditional.md).
- **学习输入习惯 (Learn typing habits)**: when off, words you pick often and words you create are no longer remembered; what's already learned still affects ranking.

## Keyboard

设置 → 键盘 (Settings → Keyboard):

- **尺寸 (Size)**: keyboard height, which you can also drag directly on the keyboard with ↕ in the toolbar; with full-screen adaptation on, a key is placed on each side of the gesture bar, and you can choose which; if both sides are set to 不放 (None), the strip goes away and the keyboard sits right above the gesture bar.
- **按键 (Keys)**: key vibration (off / light / standard / strong; standard is the original feel), key label size (small / standard / large / extra large, separate from the candidate size), key popups, and layout (default / left-handed).
- **手势 (Gestures)**: swipe typing, swipe up for symbols, long press for capitals, swipe up on delete to clear, slide space to move the cursor, hold space to talk, and keep Chinese when switching languages; see [Android: keyboard and typing](android.md).
- **游戏键盘 (Game keyboard)**: a small, semi-transparent floating keyboard in landscape in the apps you tick; the transparency, size, glosses and the list of apps are on this page; see [Android: keyboard and typing](android.md#Game-keyboard).

## Appearance

设置 → 外观 (Settings → Appearance):

- Light/dark: follow system, light or dark; the app and the keyboard change together (the app itself needs Android 12 or later; on earlier versions it follows the system). 樱花, 大肥鱼, 初音未来 and 波奇酱 are light only and stay light when dark is chosen.
- Themes: the same ten themes as on desktop (青简绿, 系统蓝, 微信绿, 素笺, 玻璃, 墨岩, 樱花, 大肥鱼, 初音未来, 波奇酱), each card showing what the candidate bar and keyboard look like. Once selected, the keyboard, candidate bar, symbol and emoji panels and quick translation bar all take on that theme's colours.
- Candidate size: small, standard, large or extra large; candidates and glosses scale together, including when all candidates are expanded, and the candidate bar gets taller with larger sizes. The labels on the keys are set separately under 设置 → 键盘 → 键面字号 (Settings → Keyboard → Key label size).

The preview at the top of the page shows the current theme, light/dark mode and size.

## Dictionaries

设置 → 词库 (Settings → Dictionaries):

- Topic dictionaries: law, medicine, place names, idioms, famous lines of poetry, IT and computing, finance, food, animals, cars, historical figures. They ship with the app and can be turned on one by one. Idioms are on by default.
- Importing dictionaries: tap 导入词库 (Import dictionary) and choose files in the system file picker; you can select several. The format is the same as pinyin dictionaries on desktop, see [Dictionaries](../personalize/dictionaries.md#Importing-your-own-dictionaries). The name and entry count are shown after importing.
- Imported dictionaries can be turned on or off individually; after 删除 (Delete) they need to be imported again. Turning on, importing and deleting take effect immediately on the keyboard in use.
- 通讯录 (Contacts): off by default; when you turn it on, the system asks whether Qingjian may read your contacts. Names of 2–6 Chinese characters can then be typed and appear on the first page with a person icon; 上屏名字后给出电话、邮箱 (Offer phone and email after a name) can be turned off separately. Names and numbers are used only on your phone; see [Privacy](../help/privacy.md#Contacts-on-Android).

## Exporting and clearing

设置 → 数据 (Settings → Data):

- Export data: saves learned words, the vocabulary book, typing statistics and saved phrases as a zip file; choose where to save it in the system's save screen. Clipboard history and imported dictionaries aren't included.
- Clear learning data: choose to clear typing habits (words you pick often, words you created, English words, your personal word order), the vocabulary book (all languages), or both; it takes effect immediately after confirming and can't be undone. Typing statistics, saved phrases, the clipboard, dictionaries and settings aren't affected.
