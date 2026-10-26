---
title: Privacy
order: 1
description: "When Qingjian goes online, what it records on your device, what the input log is for and how to turn it off, and how password fields and private input fields are handled."
---

Qingjian doesn't upload what you type. Pinyin conversion, dictionaries, learning and glosses all happen on your device. No account, no usage reporting.

## When it goes online

Only in these cases:

- **Checking for updates**: once a day, see below.
- **AI services**: off by default; when on, data goes straight from your device to the provider you entered, never through Qingjian. For what's sent, see [AI services](../cloud/index.md#What-gets-sent).
- **Voice input on Android**: while you speak, the audio is sent to the speech recognition service you entered; see [Android](../platforms/android.md#Voice-input).

### Checking for updates

Once a day, the macOS and Windows versions read the version list (a static file) from the official site qingjian.app. When there's a new version, it's announced in the input method menu (macOS), the right-click menu of the 中 / 英 icon on the taskbar (Windows) and the 关于 (About) page in Settings; nothing is downloaded or installed automatically.
The request carries no account, device identifier or anything you've typed, only what any network request necessarily carries (your IP address and Qingjian's version number). The version list is digitally signed, and anything that fails verification is ignored.

Turn off 自动检查更新 (Check for updates automatically) under 设置 → 关于 (Settings → About) and it no longer checks online; 渠道 (Channel) lets you see only stable releases or betas too.

## What it records on your device

Everything is kept in Qingjian's data folder on your own computer, as text files you can open and read:

- **Settings**. The AI service key is stored separately and is readable only by your user account.
- **Typing habits**: words you've picked, words you've created, which words go together, remembered English words and accepted corrections; see [Typing habits](../personalize/habits.md).
- **Vocabulary book**: glosses you've seen, with the matching Chinese and example sentences; see [Vocabulary book](../learning/vocabulary.md).
- **Statistics**: hourly character counts, keystrokes and so on — numbers only; see [Statistics](../learning/statistics.md).
- **Glosses filled in by AI** and AI service usage (numbers only).
- **Input log** and **runtime logs**: see below.
- Dictionaries, code tables, themes and schemes you've imported.

For the folder location and what each file contains, see [Data files and logs](../advanced/data-files.md); to delete everything, see [Uninstalling](uninstall.md).

## Password fields and private input fields

In password fields, and in input fields an app declares private (such as a browser's private window):

- typing habits aren't learned, nothing goes into the vocabulary book and nothing is written to the input log;
- nothing is sent to AI services;
- in password fields no candidates are shown and keys go straight to the app.

Whether private input fields are recognized on Linux depends on the app and the Fcitx version; see [Linux](../platforms/linux.md#Differences-from-the-desktop-versions).

## Input log

On by default in betas. Each commit records one line on your device: the keys, how the pinyin was segmented, the first page of candidates you saw, which one you picked, how many pages you turned, how long it took from the first key to the commit, and which app you were in.
A few other kinds of events get a line each: typing something different after Backspace while typing pinyin, punctuation and Enter typed directly when you weren't typing pinyin, candidates and sentences from AI, switching apps, and the input method starting.
It contains only what you typed through Qingjian, not text already in the app, and nothing from password fields or private input fields.

It's stored only on your device, to check offline how well candidates are ranked, and is never sent to anyone automatically.

- Turn it off: turn off 记录输入日志 (Record input log) under 设置 → 高级 (Settings → Advanced).
- Clear it: 清空输入日志 (Clear input log) on the same page; it needs two clicks to confirm.
- During testing, if you're willing to share this file with the author, it helps a lot with improving candidate ranking; it's entirely voluntary.

## Runtime logs

Runtime logs record how the input method itself is running, for troubleshooting, one file per day, kept for 7 days. By default they don't record what you type; only with 详细日志 (Verbose logging) on under 设置 → 高级 (Settings → Advanced) is each keystroke logged. Please turn it off when you're done troubleshooting.
For how to attach logs when reporting a problem, see [Reporting problems](feedback.md).
