---
title: Statistics
order: 3
description: "The 统计 (Statistics) page in Settings: how many characters you type and how fast, a typing calendar for the past year, vocabulary progress by level in your learning language, and AI service usage."
---

The top of 设置 → 统计 (Settings → Statistics) has two tabs, 输入 (Typing) and 云服务 (Cloud services); the cloud services tab exists only on macOS and Windows. On Android, statistics are on the home screen of the Qingjian app; see [Android: vocabulary book and settings](../platforms/android-settings.md#Vocabulary-book-and-statistics).

## Amount typed

- Four number cards: Chinese characters, Chinese words and English words typed today, in the last 7 days, this month and in total. The last 7 days include today and roll by day; they don't reset on Monday or Sunday.
- The past year: one square per day, one column per week (Sunday on top); the darker the square, the more you typed that day. Hover over a square to see the date and character count.
- The past 24 hours: one bar per hour, with the current hour on the far right; hover over a bar to see how many characters you typed in that hour.
- Typing details:
  - Your busiest day, and how many characters you type on average on days you type.
  - Speed (characters per minute): today, on average, and your fastest day. Only time spent typing counts; pauses of more than 5 seconds between keys are left out.
  - First-choice rate: when picking from the candidates, how often the pick was the first one.
  - Right first time: how often you committed without pressing Backspace first.
  - Average keystrokes: how many keys you press per Chinese character on average.
  - Your total is converted into "about n copies of *some book*", with books ranging from the *Tao Te Ching* to *Ordinary World*.

Speed, first-choice rate, right first time and keystrokes have been recorded only since 0.2.2; earlier records only have character counts.

Statistics are just numbers, recorded by hour on your device, and are unrelated to the input log: they remain after you turn off or clear the input log. Each commit is recorded on the spot, so earlier records aren't lost even if the input method is force-quit or the computer shuts down suddenly.

## Vocabulary

The number of glosses seen / familiar / committed / typed / first seen in the last 7 days, followed by the total and your current progress for each level of the word list:

- English: CEFR A1 to C2 (8,845 words in total).
- Japanese: JLPT N5 to N1 (7,757 words in total).

Levels are shown only on the statistics page, never in the candidate window. The vocabulary records contain only the glosses themselves and counts, not the Chinese you typed.

## AI service usage

The 云服务 (Cloud services) tab works like the typing tab: four number cards for today, the last 7 days, this month and in total (tokens and requests), plus tokens per day over the past year and per hour over the past 24 hours.
Tokens are the unit AI providers charge by, roughly proportional to the amount of text sent and received.
The details show your busiest day, the total number of requests, responses received, failures and requests still waiting for a result, as well as input, output and total tokens and how many of the input tokens hit the cache. The latest records are read each time you open the statistics page, and totals are kept across restarts.

- Counted: cloud suggestions, automatic gloss filling, sentence translation for the vocabulary book, translating selected text and 测试连接 (Test connection). Every request actually sent counts, and an automatic retry counts separately; requests cancelled before sending and results served from the cache don't count.
- When a provider doesn't report usage, it's recorded as "unknown".
- The daily and hourly charts exist only since 0.2.2; earlier usage appears only in the totals. Usage from before recording started can be checked on the provider's website.
