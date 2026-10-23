---
title: AI services (optional)
order: 5
description: "Optional AI services, off by default: what they can do, how to turn them on and enter a provider, what gets sent, and costs and usage records."
---

Qingjian's typing, dictionaries, learning and glosses all run on your device without going online. AI services are an optional extra, **off by default**, and you need to enter an AI provider yourself (such as DeepSeek).
Data goes straight from your device to the provider you entered and never passes through Qingjian: Qingjian has no servers of its own.

## What they can do

| Feature | Description | Platforms |
|---|---|---|
| [Cloud suggestions and sentence completion](predict.md) | Adds a few AI candidate words while you type pinyin, plus one complete sentence | macOS, Windows, Linux |
| [Asking about a character](question-and-translate.md#Asking-about-a-character) | Ask in pinyin, e.g. "which character is three 木?" | macOS, Windows, Linux |
| [Translating selected text](question-and-translate.md#Translating-selected-text) | Translates text selected in an app into your learning language or back into Chinese | macOS, Windows, Linux |
| [Filling in glosses automatically](sentence-translation.md#Filling-in-glosses-automatically) | Adds a gloss for words that have none in the glossary | macOS, Windows |
| [Sentence translation for the vocabulary book](sentence-translation.md#Sentence-translation-for-the-vocabulary-book) | Translates finished sentences and records them in the vocabulary book | macOS, Windows |
| [Translating sentence candidates](sentence-translation.md#Translating-sentence-candidates) | Shows a translation to the right of the sentence candidate | Windows |

For AI features on Android, see [Android: tools and AI](../platforms/android-tools.md).

## Turning them on

1. Sign up with an AI provider on its website and get an API key. Any service compatible with the OpenAI API works; DeepSeek's address is filled in by default.
2. Open 设置 → 云服务 (Settings → Cloud services) and fill in 接口地址 (API address), 模型 (Model) and API 密钥 (API key). Fields are saved when you press `Enter` or leave them; `Esc` discards the change. The key field doesn't show a key that's already saved.
3. Click 测试连接 (Test connection): it sends the shortest possible request with what you entered and shows the model name and response time on success, or the reason on failure.
4. Turn on 启用 (Enable) and the features you want.

You can fill in and test the settings while they're off, and turn them on once the test passes. On macOS, a ☁︎ after 中 / 英 in the menu bar means they're on; the input method menu also has a switch.
On Linux you can also turn them on by editing the configuration file directly; see [Editing the configuration file](../advanced/config-file.md#AI-services).

- The key is stored only on your device; it isn't included when the configuration file is exported, and it doesn't appear in the log bundle used for reporting problems.
- The input method is started by the system and doesn't read proxy settings from your terminal; if you need a proxy to reach the provider, set a system proxy or use a proxy tool in TUN mode.
- Using it incurs the provider's charges; see [Statistics](../learning/statistics.md#AI-service-usage) for how much you've used.

## What gets sent

- **Cloud suggestions**: the pinyin you're typing, plus the segmentation and candidates produced locally. On macOS and Linux, when the app provides it, the text before and after the cursor may also be included (on Linux, at most 80 characters on each side, excluding selected text); Windows doesn't include it.
- **Asking about a character**: the question you typed in pinyin.
- **Translating selected text**: the selected text (up to 500 characters), read and sent only when you press the translation shortcut.
- **Filling in glosses automatically**: the one word missing a gloss.
- **Sentence translation for the vocabulary book**: committed sentences (two characters or more) or sentences finished in several parts (four or more Chinese characters), sent a few at a time; each sentence is sent only once.
- **Translating sentence candidates**: the current sentence candidate and the target language; no pinyin, no text from the app and no other candidates.

Nothing is ever sent from password fields or input fields an app marks as private. On Linux, Fcitx4 can only recognize password fields, not other private input fields, so take care when using it.
Once AI services are turned off, no more requests are sent.
