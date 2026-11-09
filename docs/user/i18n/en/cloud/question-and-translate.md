---
title: Asking and translating
order: 2
description: "Asking a question in pinyin, such as which character is made of three 木, translating text selected in an app into your learning language or back into Chinese, and translating as you type in the Translate group of the symbol panel. All require AI services."
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
- You can enter a question without AI services on, but you won't get an answer; `Enter` outputs the text as typed. Naming components (`umumu`) finds the character locally without AI services; see [Finding a character by its components](../input/shortcuts.md#Finding-a-character-by-its-components).

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

## Translating in the symbol panel

On macOS and Windows the [symbol panel](../input/punctuation.md#Symbol-panel) has a 翻译 (Translate) group: type a sentence of Chinese in its box, pause, and the translation appears at the cursor in the app; press `Enter` to commit it.

- **Opening**: while typing pinyin, press `↑` to open the panel; it lands on the Translate group and carries the pinyin into the box. When not typing, press `⌃⌥Y` (Windows: `Ctrl + Alt + Y`). Opening the panel from the menu or the status bar returns to the group you used last.
- **Typing in the box**: type pinyin as usual; it shows at the cursor in the box, and `Space`, a number key or clicking a candidate in the box puts the word at the cursor, not into the app. Punctuation is converted to full width as usual. In English mode, what you type goes straight into the box.
- **Editing the box**: like Notepad, clicking places the cursor, dragging selects text, and `Shift`-clicking extends the selection to that point; when not typing pinyin, `←` `→`, `Home` and `End` move the cursor, with `Shift` held they select, and `⌘ + A` (`Ctrl + A` on Windows) selects all. Typing or picking a word replaces the selected text, and `Backspace` / `Delete` remove it; with nothing selected, `Backspace` deletes the character before the cursor and `Delete` the one after. When the text grows, the box scrolls so the cursor line stays visible.
- **Target language**: `[` `]` switch the target language (English, Japanese, Spanish, Vietnamese, German). It starts as your learning language and the last one you chose is remembered. If the box holds foreign text, it is translated back into Chinese, decided the same way as in the previous section.
- **When it's sent**: only after you pause for about 0.8 seconds with two or more characters in the box, and not again if nothing changed. Deleting and retyping, or switching to another language and back, reuses translations remembered on your device instead of sending again; they're kept in memory only and are gone when the input method quits.
- **Committing and discarding**: once the translation matches the box, `Enter` commits it; pressing `Enter` before it arrives sends right away, and you press it again when it's there. `Esc` clears the box and the translation in the app; with an empty box, `Esc` goes back to the candidates.
- **When no translation comes back** (usually a network hiccup): it's sent again automatically after 2 seconds; if that fails too, the bottom line says to press Enter to retry, and `Enter` sends it again.

The shortcut can be changed under 设置 → 快捷键 (Settings → Shortcuts); on Windows, switch to another input method and back to Qingjian (or reopen the app) for the change to take effect. With AI services off, or in a password or private input field, the panel has no Translate group.
Some apps (certain web editors, terminals) don't display uncommitted text at the cursor well, and a long translation may look messy; the line at the bottom of the box shows the progress.
