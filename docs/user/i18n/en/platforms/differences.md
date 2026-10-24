---
title: Platform differences
order: 1
description: "Which Qingjian features are available on macOS, Windows, Linux and Android: current status, key conventions and what isn't supported yet."
---

The macOS and Windows versions are in beta with nearly the same features; keys follow each system's conventions. The Linux version uses Fcitx's candidate panel and is started by hand. The Android version is a preview with fewer features, installed from an APK.

"✓" means supported; "—" means not supported yet, or not applicable.

## Typing

| Feature | macOS | Windows | Linux | Android |
|---|---|---|---|---|
| Full pinyin, sentences, abbreviated pinyin, spelling correction | ✓ | ✓ | ✓ | ✓ |
| 9-key keypad | — | — | — | ✓ |
| Double pinyin, fuzzy pinyin | ✓ | ✓ | ✓ | ✓ |
| Dachen Zhuyin | — | ✓ | ✓ | ✓ |
| Wubi, Wubi–pinyin mixed input | ✓ | ✓ | — | ✓ |
| Custom double pinyin / shape-based schemes | ✓ | ✓ | — | — |
| Auxiliary code | ✓ | ✓ | — | — |
| Key hints and radical chart | ✓ | ✓ | — | — |
| Local sentence model | ✓ | ✓ | ✓ | — |
| English mode and English completion | ✓ | ✓ | ✓ | ✓ (English keyboard) |
| Choice of Chinese/English switch key | — (`Caps Lock`) | ✓ | ✓ | — |
| Start in English mode | — | ✓ | ✓ | — |
| Remember mode per app | — | ✓ | — (each window keeps its own) | — |
| Automatic spacing between Chinese and Latin text | ✓ | ✓ | — | — |
| Symbol panel | ✓ | ✓ | — | ✓ (symbol keyboard) |
| Dates, calculations, characters by code point | ✓ | ✓ | ✓ | ✓ |
| Traditional Chinese output | ✓ | ✓ | ✓ | ✓ |
| Custom phrases | ✓ | ✓ | ✓ | ✓ (saved phrases) |
| System text replacements | ✓ | — | — | — |
| Importing pinyin dictionaries | ✓ | ✓ | ✓ | ✓ |
| Importing Wubi dictionaries | ✓ | ✓ | — | — |
| Voice input, swipe typing, clipboard | — | — | — | ✓ |

## Learning languages

| Feature | macOS | Windows | Linux | Android |
|---|---|---|---|---|
| Glosses beside candidates and new-word marks | ✓ | ✓ | ✓ (first gloss only) | ✓ |
| Committing glosses directly | ✓ | ✓ | ✓ | ✓ (long-press a candidate) |
| Vocabulary book, statistics | ✓ | ✓ | ✓ | ✓ |

## Appearance

| Feature | macOS | Windows | Linux | Android |
|---|---|---|---|---|
| Seven built-in themes, light and dark | ✓ | ✓ | — (Fcitx panel) | ✓ |
| Custom themes | ✓ | ✓ | — | — |
| Expanding horizontal layout into rows | ✓ | ✓ | — | — |
| Tones on the pinyin line | ✓ | ✓ | — | — |
| Floating status bar | — | ✓ | — | — |

## Keys and shortcuts

| Feature | macOS | Windows | Linux | Android |
|---|---|---|---|---|
| Changing paging keys | ✓ | ✓ | ✓ | — |
| Extra paging keys, highlight keys | ✓ | ✓ | — | — |
| Full-width/half-width punctuation shortcut | ✓ | ✓ | — | — |

## AI services

| Feature | macOS | Windows | Linux | Android |
|---|---|---|---|---|
| Cloud suggestions and sentence completion | ✓ | ✓ | ✓ | ✓ |
| On-demand suggestions, request timing | ✓ | ✓ | ✓ | — |
| Asking about a character | ✓ | ✓ | ✓ | — |
| Translating selected text | ✓ | ✓ | ✓ | ✓ |
| Quick translation bar | — | — | — | ✓ |
| Filling in glosses automatically | ✓ | ✓ | ✓ | ✓ |
| Sentence translation for the vocabulary book | ✓ | ✓ | ✓ | — |
| Translating sentence candidates | — | ✓ | — | — |
| AI usage statistics | ✓ | ✓ | ✓ | — |

Installation and platform-specific features: [Installation](../getting-started/install.md) (macOS, Windows), [Linux](linux.md), [Android](android.md).
