---
title: Vocabulary book
order: 2
description: "The 生词本 (Vocabulary book) page in Settings: a daily timeline and a word list, filtering and search, marking words as known, exporting to a spreadsheet or Anki, and clearing."
---

The 生词本 (Vocabulary book) page in Settings records the Chinese you've typed and how it's said in your learning language, in two views: 时间线 (Timeline) and 词表 (Word list). It's stored only on your device; text typed in password fields and in input fields an app marks as private isn't recorded.

## Timeline

Arranged by day, one entry per commit: the learning-language version in large text on top, the Chinese below, and the time on the right.

- When you commit Chinese (two characters or more) that has a gloss, its first gloss is recorded; when you commit a gloss directly, the gloss you committed is recorded. Japanese also shows the kana reading.
- Whole sentences don't have ready-made glosses. On macOS and Windows, with AI services on, sentences are translated in the background and added to the timeline, usually within a few seconds; see [AI translation](../cloud/sentence-translation.md#Sentence-translation-for-the-vocabulary-book). With AI services off, or that option off, sentences aren't recorded.
  A sentence committed in one go, and a sentence committed in several parts that has four or more Chinese characters when it's finished (by end-of-sentence punctuation, switching apps or reaching 80 characters), are both translated as sentences.
- The top shows how many words you met that day, how many of them were new, how many days in a row you've been typing, and an hour-by-hour "memory strip". Use the arrows beside the date to go to the previous or next day with records.
- An entry for a single word comes with an example sentence: the sentence it was in, from the previous full stop, question mark, exclamation mark or line break to the next one; if you never type a full stop, until you switch apps or reach 80 characters.

## Word list

One gloss per row, with the Chinese, an example sentence and how many times it was seen and committed.

- **Filter and sort**: all, new, first seen in the last 7 days, known; sort by most recently seen, count or alphabetically. The search box searches glosses, Chinese and example sentences. 50 words per page.
- **认识了 (Known)**: once marked, the gloss is no longer shown as new among candidates; you can unmark it any time.
- **Delete**: removes the word from the list; it's recorded again the next time you meet it.
- English words recorded before the vocabulary book existed have no Chinese; the list fills it in from the bundled English–Chinese glossary.

## Shared by both views

- **Language**: each learning language is recorded separately, and the current learning language is shown when you open the page; if you've recorded several languages, you can switch at the top to view ones you studied before.
- **Export**: exports the words currently filtered and searched in the word list as CSV (open it in a spreadsheet), an Anki import file (gloss on the front, Chinese and example sentence on the back) or a web page (open it in a browser, print it).
- **Clear**: deletes all records for the language you're viewing, timeline included; you need to click twice to confirm.

On Android the vocabulary book is in the Qingjian app; see [Android: vocabulary book and settings](../platforms/android-settings.md#Vocabulary-book-and-statistics).
