---
title: Cloud suggestions and sentence completion
order: 1
description: "Pause briefly while typing pinyin and AI adds a few candidate words plus a complete sentence; on-demand suggestions, request timing, and when no suggestions are made."
---

AI services must be turned on first; see [AI services](index.md). It takes effect once 云联想 (Cloud suggestions) is on under 设置 → 云服务 (Settings → Cloud services).

## AI candidate words

Pause briefly while typing pinyin (300 milliseconds by default) and the words AI suggests are added to the **last** two slots of the first page of candidates (adjustable from 0 to 4 on the 云服务 (Cloud services) page). They're marked with a cloud, number keys pick them as usual, and picking one uses up the whole stretch of pinyin.

- The local candidates in front stay where they are, and `Space` still commits the local first candidate.
- If nothing comes back, there's no result or the request times out, the display doesn't change; no space is reserved and no placeholder is drawn.
- AI words you pick are remembered (by the reading you typed), so next time they're offered without going online.
- AI can correct typos: `zhgdoima` also gives 这个东西吗.
- When you type only initials (abbreviated pinyin, such as `fhyq`), no words are suggested, only a sentence completion: words guessed from initials are mostly not real words.
- After you move the highlight or change page, AI words that arrive late aren't inserted, so the slot you're choosing from doesn't move.

## Sentence completion

The same request also returns one sentence: a complete phrasing that starts with what you've typed (`suoyiwoxiangq` → 所以我想去吃饭), shown with a cloud to the right of the pinyin (on Linux, in the hint line below the candidate panel).
`Tab` accepts it; typing more replaces it. When the app provides the text before the cursor, the completion continues from it.

## On-demand suggestions

On macOS and Windows you can turn automatic suggestions off and ask only when you need them: while typing pinyin, press `Ctrl + Alt + J` (`⌃⌥J` on macOS) and the AI results go to the front of the candidates; automatic suggestions still go at the end of the first page.

- 自动联想 (Automatic suggestions) can be turned off under 设置 → 云服务 (Settings → Cloud services).
- Change the key combination under 按需 AI 联想 (On-demand AI suggestions) in 设置 → 快捷键 (Settings → Shortcuts); it takes effect immediately.

## Request timing

On macOS and Windows, the 云服务 (Cloud services) page lets you adjust:

- **停键等待 (Pause before request)**: how long after you stop typing the request is sent; 300 milliseconds by default.
- **最短请求间隔 (Minimum interval)**: the minimum time between two requests; 3000 milliseconds by default. If you keep typing within that interval, only the latest pinyin is kept.

Picking a word, clearing or switching to a private input field cancels requests not yet sent; the same input reuses the last result instead of going online again.
Ordinary failures wait for the minimum interval and then retry; an invalid key or insufficient balance pauses requests for 10 minutes.

## When no suggestions are made

No suggestions after you commit; none in English mode or for calculations starting with `v`; none in password fields or input fields an app marks as private.
