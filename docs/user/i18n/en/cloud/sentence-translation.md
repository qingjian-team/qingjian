---
title: AI translation
order: 3
description: "Using AI to fill in glosses and sentence translations: filling in missing glosses automatically, translating sentences into the vocabulary book, and showing a translation beside sentence candidates (Windows)."
---

The bundled glossary only has glosses for words. With AI services on, the three options below fill in what's missing; see [AI services](index.md).

## Filling in glosses automatically

On macOS and Windows, a word with no gloss in the glossary is asked about once on its own after you commit it (only that word is sent); the gloss that comes back is stored on your device, shown beside the candidate next time, and recorded in the vocabulary book too.
Nothing is sent when the learning language is set to 不显示 (Don't show) or in a private input field. Not yet available on Linux.

## Sentence translation for the vocabulary book

Chinese committed as a whole sentence has no ready-made gloss. On macOS and Windows, with 生词本整句翻译 (Translate sentences for the vocabulary book) on under 设置 → 云服务 (Settings → Cloud services) (on by default),
finished sentences are collected and sent a few at a time for translation; the translations are recorded only in the timeline of your local [vocabulary book](../learning/vocabulary.md#Timeline) and aren't shown among the candidates.

- What counts is a sentence committed in one go (two characters or more), and a sentence committed in several parts that has four or more Chinese characters when finished.
- Each sentence is sent only once. If you don't want sentences sent, turn this option off; cloud suggestions and the other features keep working. Not yet available on Linux.

## Translating sentence candidates

Currently Windows only. With 整句候选翻译 (Translate sentence candidates) on under 设置 → 云服务 (Settings → Cloud services), a translation appears to the right of the local sentence candidate (a sentence of Chinese converted from a string of pinyin). It doesn't change the candidate order and doesn't produce AI candidate words or sentence completions.

- Click the translation to commit it, or press `Ctrl + number` to commit the translation of that candidate.
- Only the current sentence candidate and the target language are sent each time; no pinyin, no text from the app and no other candidates. Nothing is sent from password fields or private input fields.
- When the input keeps changing, results of old requests are discarded; committing, deleting all the pinyin or pressing `Esc` cancels requests not yet sent.
