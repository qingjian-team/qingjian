---
title: AI translation
order: 3
description: "Using AI to fill in glosses and sentence translations: filling in missing glosses automatically and translating sentences into the vocabulary book."
---

The bundled glossary only has glosses for words. With AI services on, the two options below fill in what's missing; see [AI services](index.md). To translate a sentence you type yourself, use the [Translate group](question-and-translate.md#Translating-in-the-symbol-panel) of the symbol panel.

## Filling in glosses automatically

On macOS and Windows, a word with no gloss in the glossary is asked about once on its own after you commit it (only that word is sent); the gloss that comes back is stored on your device, shown beside the candidate next time, and recorded in the vocabulary book too.
Nothing is sent when the learning language is set to 不显示 (Don't show) or in a private input field. Not yet available on Linux.

## Sentence translation for the vocabulary book

Chinese committed as a whole sentence has no ready-made gloss. On macOS and Windows, with 生词本整句翻译 (Translate sentences for the vocabulary book) on under 设置 → 云服务 (Settings → Cloud services) (on by default),
finished sentences are collected and sent a few at a time for translation; the translations are recorded only in the timeline of your local [vocabulary book](../learning/vocabulary.md#Timeline) and aren't shown among the candidates.

- What counts is a sentence committed in one go (two characters or more), and a sentence committed in several parts that has four or more Chinese characters when finished.
- Each sentence is sent only once. If you don't want sentences sent, turn this option off; cloud suggestions and the other features keep working. Not yet available on Linux.
