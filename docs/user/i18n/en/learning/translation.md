---
title: Glosses beside candidates
order: 1
description: "The language and format of the gloss beside each candidate, what orange new words mean, and how to commit the gloss directly instead of the Chinese."
---

## Glosses

Each candidate has at most one gloss on its right, with the part of speech. Choose the language under 译文 (Gloss) in 设置 → 候选 (Settings → Candidates): English, Japanese, Spanish, Vietnamese or German, one language at a time.

- English: `开发` → v. develop · v. exploit.
- Japanese marks the kanji with hiragana: `开发` → v. 開発(かいはつ)する.
- Spanish: `开发` → v. desarrollar.
- Vietnamese: `开发` → v. phát triển.
- German: `房子` → n. das Haus.

- With 译词显示词性 (Show part of speech) turned off on the same page, only the gloss is shown beside the candidate, without v. / n. / adj. labels (`develop` instead of `v. develop`).
- Choosing 不显示 (Don't show) shows no glosses at all, and new-word marking and the vocabulary book stop too; translating selected text still works (into English).
- Candidates with no gloss are left blank, with no placeholder. The bundled glossary covers the whole dictionary; with AI services on, words missing from it can also [get a gloss filled in automatically](../cloud/sentence-translation.md#Filling-in-glosses-automatically).
- The Linux candidate panel shows only the first gloss.

## New words

**Orange glosses are new words**: the gloss has been in the candidate window fewer than three times when you committed text. After three times it turns grey on its own, with no icons and no pop-ups.
Only the page on screen at the moment you commit counts; glosses that flash by while you're typing don't. Words you mark as 认识了 (Known) in the vocabulary book are no longer shown as new; see [Vocabulary book](vocabulary.md).

## Committing the gloss directly

While typing pinyin, you can commit the gloss next to a candidate instead of the Chinese:

| Action | macOS | Windows | Linux |
|---|---|---|---|
| Commit the first gloss of candidate N | `⌥ + 1` to `9` | `Ctrl + 1` to `9` | `Alt + 1` to `9` |
| Commit the second gloss (when the entry has one) | `⇧⌥ + 1` to `9` | `Ctrl + Shift + 1` to `9` | `Alt + Shift + 1` to `9` |

For example, type `nihao`, and when the candidate 你好 shows `hello` beside it, press `⌥ + 1`: what you get is `hello`.

- On macOS and Windows you can also click the gloss to the right of a candidate; whichever gloss you click is committed.
- Only the gloss itself is committed, without the part of speech, the Japanese kana reading or the new-word mark: `v. 開発(かいはつ)する` outputs just `開発する`. A gloss committed right after a word in a language written with spaces, such as English or Spanish, gets a space in front automatically (`hello` then `world` gives `hello world`); Japanese glosses don't. This does not depend on the 中文与英文、数字之间自动加空格 (automatic spacing between Chinese and Latin text) switch.
- The keys do nothing when a candidate has no gloss. AI services don't need to be on.
- The modifier keys can be changed under 设置 → 快捷键 (Settings → Shortcuts); see [Changing keys](../personalize/keys-settings.md#Glosses-and-deleting-candidates).

## Related

- Every gloss you've seen while typing is recorded in the [vocabulary book](vocabulary.md), and your learning progress is in [Statistics](statistics.md).
- To translate a passage you've selected in an app into your learning language, see [Asking and translating](../cloud/question-and-translate.md#Translating-selected-text).
