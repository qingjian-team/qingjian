---
title: Typing habits
order: 5
description: "What Qingjian remembers from your typing, how old habits slowly fade, pinning, demoting and deleting candidates from the right-click menu, turning learning off and starting over."
---

Qingjian remembers your typing habits so that words you use often rank higher. Everything is recorded on your device and never uploaded.

## What it remembers

- Words you've picked, and which word you picked for a given stretch of pinyin; a word picked by its full pinyin also counts as picked for its abbreviation (pick 屎山 for `shishan` and it comes first for `ss` too).
- Which words you tend to type together, which sentence conversion adjusts to.
- Words you've created: when you pick a stretch of pinyin in several steps (`shishan`: first 屎, then 山) and the result isn't in the dictionary, it's saved as one word right away, and next time it comes first for `shishan` and for the abbreviation `ss`; whole sentences work the same way (up to 20 characters): after picking 晓铭去了学校 piece by piece for `xiaomingqulexuexiao`, the same full pinyin or the sentence abbreviation `xmqlxx` brings it up first. A sentence picked in one go is also saved as a word when it's made of words of two or more characters, 6 characters or fewer in total, and not in the dictionary: pick 交易中心 for `jiaoyizhongxin` once, and next time `jyzx` brings it up first; if you delete it right away and pick something else instead, the word isn't kept. Two words picked one after the other across separate inputs are saved as one word after you've picked them a few times.
- AI candidates you've picked (recorded by the reading you typed).
- English words you've committed as typed.
- Spelling corrections you've accepted; see [Sentences, abbreviations and correction](../input/sentence.md#Remembering-habitual-typos).

If you pick the wrong word, delete it and retype, that pick isn't remembered; see [Picked the wrong word](../getting-started/first-input.md#Picked-the-wrong-word).

## Slowly fading

Pick counts cool off naturally over time: if you don't type a word for 90 days, its count is roughly halved, so old words make way for new ones and the candidates always follow your current habits.
Cooling only affects ranking; the records themselves stay.

## Candidate right-click menu

While typing pinyin, right-click a candidate (on macOS you can also Control-click). Depending on the candidate, the menu offers these items, which apply only to the letters you're typing now:

- **修改译文… (Edit translation…)**: correct the candidate's translation in your current learning language. The personal glossary overrides the bundled glossary and takes effect immediately.
- **模型重新生成译文 (Regenerate translation with AI)**: asks your configured cloud model to translate the candidate into the current learning language, then saves the result to your personal glossary. This is shown only when both a learning language and the cloud service are enabled; no request is sent from private input fields.
- **置顶 (Pin to top)**: from now on, typing the same letters puts this word first. It adds a top-weight entry to 自定义短语 (Custom phrases), where you can see it, change its weight or delete it; see [Custom phrases](phrases.md). Only the exact same letters count: abbreviated and full pinyin are separate.
- **取消置顶 (Unpin)**: deletes that custom phrase.
- **降权 (Demote)**: moves the word back one place; click again to move it further. Dictionary words can be demoted too.
- **取消降权 (Undo demote)**: puts it back where it was.
- **删除 (Delete)**: words you've created and AI candidates you've picked are deleted entirely; dictionary words aren't deleted, but all records about them are cleared and their original ranking is restored.

The result is shown briefly beside the pinyin line (such as 已删除用户词「…」 "user word deleted") and disappears with the next key. While the menu is open, `Esc` only closes the menu.
Available on Windows and macOS; not yet on Linux.

## Turning learning off and starting over

- With 学习输入习惯 (Learn typing habits) turned off under 设置 → 候选 (Settings → Candidates), nothing new is recorded, though what's already remembered still affects ranking.
- Nothing typed in password fields or in input fields an app marks as private is ever recorded.
- To start over, delete the typing-habit files in the data folder; see [Data files and logs](../advanced/data-files.md#Data-files). On Android you can clear everything with one tap in the Qingjian app; see [Android: vocabulary book and settings](../platforms/android-settings.md#Exporting-and-clearing).
