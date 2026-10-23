---
title: Asking and translating
order: 2
description: "Asking a question in pinyin, such as which character is made of three 木, and translating text selected in an app into your learning language or back into Chinese. Both require AI services."
---

AI services must be turned on first; see [AI services](index.md).

## Asking about a character

In Chinese mode, start with `u` and type your question in pinyin:

- `usangemushishenmezi` (三个木是什么字, "which character is three 木?")
- `umumumu` (naming the components)
- `ufaguoshoudu` (a short factual question)

After a pause, the answer character or a short answer comes back, marked with a cloud and its toned reading (森 sēn); pick it with a number key to commit.

- When everything after `u` is exactly an English word (`unchanged`, `us`), it isn't treated as a question: English candidates are shown directly and nothing goes online. Keep typing into something that isn't an English word (`usangemu`) and it's asked as usual.
- Words not in the English word list (such as `uber`) still go to the question: type one once in English mode and it's remembered, after which Chinese mode gives English candidates for it directly too. If you rarely ask about characters, you can also change the prefix to `i` under 设置 → 快捷键 (Settings → Shortcuts).
- With double pinyin and Wubi–pinyin mixed input, `u` is a code key, so ask with `Shift + U` followed by pinyin.
- You can enter a question without AI services on, but you won't get an answer; `Enter` outputs the text as typed.

### Asking with a question mark

By default `?` is just a question mark. With ? 也进问字 (? also asks about a character) on under 设置 → 快捷键 (Settings → Shortcuts), typing `?` when you're not typing pinyin, followed by pinyin, also starts a question, in both Chinese and English mode.
If what follows the `?` isn't a letter, it goes back to being a plain question mark: `Space` and `Enter` output just the question mark, punctuation such as a comma after it is output normally, and `Backspace` deletes it.

## Translating selected text

Select some text in an app (up to 500 characters) and press `⌃⌥T` (Windows: `Ctrl + Alt + T`; Linux: `Alt + Shift + T`):

- Text that's mostly Chinese characters is translated into your learning language; text with at least as many kana or Latin letters as Chinese characters is translated back into Chinese. If the learning language is set to 不显示 (Don't show), it's translated into English.
- The candidate window shows 翻译中… (Translating…) first, then the translation as the only candidate.
- `Enter` / `Space` / `1` replace the selected text with the translation; `Esc` keeps the original; any other key abandons the translation and the key goes to the app as usual.

When AI services are off, in a password field, or when the app doesn't provide the selected text (some apps), the shortcut is passed to the app unchanged.
The shortcut can be changed under 设置 → 快捷键 (Settings → Shortcuts); on Windows, after changing it, switch to another input method and back to Qingjian (or reopen the app) for it to take effect.

Limitations on Linux:

- The app has to provide the selected text to Fcitx. Under X11, Fcitx4 can also read the current app's system selection; native Wayland apps only support the former.
- If you upgraded from an earlier version and your configuration file still has the old translation shortcut, installing the update doesn't change it; change it to `Alt + Shift + T` yourself, see [Editing the configuration file](../advanced/config-file.md#AI-services).

On Android, tap 青简翻译 (Qingjian Translate) in the system's text selection menu; see [Android: tools and AI](../platforms/android-tools.md#Translating-selected-text).
