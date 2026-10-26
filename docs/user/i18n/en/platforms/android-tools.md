---
title: "Android: tools and AI"
order: 4
description: "Qingjian for Android: clipboard and saved phrases, quick translation, translating selected text, gloss filling and cloud suggestions."
---

## Clipboard and saved phrases

- The clipboard icon in the toolbar opens the clipboard and saved phrases. Text and images you've copied are stored on the phone; tap one to commit it. Passwords copied from a password manager aren't stored.
- A clipboard entry can be added as a saved phrase, or you can use 拆词 (Split into words) to pick part of it word by word.
- Saved phrases can be grouped and given an input code; type the code to get that phrase directly among the candidates.
- 键盘设置 → 剪贴板 (Keyboard settings → Clipboard) lets you turn recording off or set the maximum number of entries kept.

## Setting up AI services

Quick translation, translating selected text, filling in glosses and cloud suggestions all use AI services: in the Qingjian app, under 我的 → 键盘设置 → 云端大模型 (Me → Keyboard settings → Cloud LLM), enter the API address, model and API key of an OpenAI-compatible service, and tap 测试连接 (Test connection) to check.
The key is stored encrypted on the phone and can't be recovered after Qingjian is uninstalled. Using it incurs data usage and the provider's charges.

## Quick translation

The translation icon in the toolbar opens the quick translation bar. Type the original text in the translation box and the translation appears live in the input field outside; tap 确定 (OK) or Enter to keep it, or 取消 (Cancel) to withdraw it.
Chinese is translated into your learning language, and other languages into Chinese. Text typed in the translation box doesn't count toward today's character count.

## Translating selected text

After selecting text in any app, tap 青简翻译 (Qingjian Translate) in the system's text selection menu (some systems put it under More); a translation card pops up at the bottom of the screen with the original on top and the translation below. Chinese is translated into your learning language, and other languages into Chinese; if the learning language is set to none, it's translated into English.

- 复制 (Copy): puts the translation on the clipboard.
- 替换 (Replace): replaces the selected text with the translation. Only shown in editable input fields.
- If translation fails, the card shows a red 翻译失败 (Translation failed) notice with the reason, without Copy and Replace; tap 重试 (Retry) to try again.
- Tap outside the card or press Back to close it.

If AI services aren't set up, the card has a 去设置 (Go to settings) button.

## Filling in glosses

A few words have no gloss in the glossary, leaving the space beside the candidate empty. With 补充译词 (Fill in glosses) on under 云端大模型 (Cloud LLM), committing such a word asks AI to write a gloss for it, which is stored on the phone; the next time you type the word, the gloss appears beside the candidate and is recorded in the vocabulary book too.
Only the word itself is sent. Off by default, and can't be turned on until AI services are set up. Nothing is sent when the learning language is set to none or in private input fields.

## Cloud suggestions and sentence completion

With 云联想与整句补全 (Cloud suggestions and sentence completion) on under 云端大模型 (Cloud LLM), pause briefly while typing pinyin and Qingjian sends the pinyin and a short piece of text around the cursor to AI; the words and sentences it returns come after the first candidate, marked with a small cloud and shown in cyan, including when all candidates are expanded. Tap a sentence to commit it.

- Works on the 9-key keypad too; what's sent is the pinyin shown on the pinyin line.
- 只在 Wi-Fi 下使用 (Wi-Fi only) is on by default: nothing is sent on mobile data. Quick translation and filling in glosses aren't affected.
- Off by default, and can't be turned on until AI services are set up. Nothing is sent from private input fields.
