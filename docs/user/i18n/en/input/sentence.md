---
title: Sentences, abbreviations and correction
order: 1
description: "Typing whole sentences, abbreviated pinyin with initials only, correcting mistyped letters and remembering habitual typos, and the local sentence model that ranks sentences on your device."
---

## Sentences

There's no mode to switch: type a whole sentence and then commit it. The first candidate for `jintiantianqihenhao` is 今天天气很好.
When you reach the last character of a sentence, typing just its initial is enough to get the whole sentence early: the first candidate for `youdiandongx` is 有点东西, the same as for the full `youdiandongxi`.

When you pick a stretch of pinyin in several steps, the words picked first stay in the input box and the whole stretch is committed together once it's done; see [Picked the wrong word](../getting-started/first-input.md#Picked-the-wrong-word).

## Abbreviated pinyin

Typing only each character's initial works too: `jttqhh` gives 今天天气很好.
Abbreviations of two or three letters are usually still treated as words (`sj` gives 时间); type a few more syllables when you want a sentence.

## Spelling correction

- **When the pinyin doesn't spell anything valid** (an extra letter, a missing letter, two keys swapped, one wrong letter): `nihooma` → 你好吗, `woxiagnqu` → 我想去, `mingtain` → 明天.
  The pinyin line marks the letters being changed with a strikethrough and keeps them in place; all candidates are based on the corrected pinyin.
  You don't have to finish the word when two neighbouring keys are swapped: `mignt` and `migntia` already give 明天, so just keep typing.
  When both kinds of correction make sense, a swap is preferred (`mignti` → 命题, rather than 面体 by changing one letter).
- **When every syllable is valid but the sentence makes no sense**: `meiganxi` still gives 没关系 (`gan` is read as `guan`). This kind of correction only affects sentences, not single-word candidates.

A correction is only made when the corrected sentence is clearly more natural than the original; valid abbreviations (`nhao`, `womend`) aren't "corrected" by mistake, and input that is entirely an English word (`hello`) isn't corrected either.

### When you don't want a correction

`Enter` still commits the letters exactly as typed. A letter string you've committed as typed once is never corrected again.

### Remembering habitual typos

Corrections you accept are remembered: if you habitually type `shuo` as `shou`, after a few times 我说 will naturally move up. Deleting with `Backspace` and picking something else also undoes that record.

There's no correction with double pinyin or Wubi: a wrong key in double pinyin changes a whole initial or final, and Wubi keys are radical codes, so neither is a case of misspelling one letter.

## Local sentence model

The local sentence model is off by default: sentences are built from the dictionary only, which is fastest and uses the least power. To use one, download one of two models under the experimental features below: Hanzhang Tongbian writes whole sentences straight from the pinyin you typed so that the first candidate for long sentences reads more naturally; Zhiyi reads the characters before the cursor and picks the best first candidate. Both run on your device: no network, nothing sent anywhere. The Linux version bundles Tongbian; turn on 本地整句模型 (Local sentence model) under 设置 → 候选 (Settings → Candidates) to use it.

- Pause briefly after typing (about a fifth of a second for long sentences) and the sentence candidate is replaced with the one the model finds more natural; until you pause, the sentence built from the dictionary is shown.
- For long input (from about six characters), the sentence Tongbian writes comes first. For short input, when the last syllable isn't finished yet, or when a typo was corrected, the dictionary's sentence comes first and Tongbian's follows. It also fills in a sentence when you mix Chinese and English or the dictionary can't build a full one.
- The model only adjusts sentence candidates. Word candidates and words you've picked before aren't affected, and your own habits still come first.
- Once you've paged further or moved the highlight with the arrow keys, the model's result no longer replaces the page you're looking at.
- For the first few seconds after the input method starts, the model is still loading and sentences come from the dictionary.
- On older Macs (for example, macOS 14 without the required graphics features), the model runs on the processor instead and ranking may be a little slower.
- Hanzhang Tongbian looks only at the pinyin you typed; Zhiyi also reads the few dozen characters before the cursor. When an app doesn't share its text (some terminals and Electron apps), they use what you just typed in that text box instead: switching apps or text boxes starts over, and text older than a minute without typing no longer counts. That time is `context_memory_seconds` in the `[model]` section of the configuration file; 0 means always count it.

To stop using it, turn the model off under 设置 → 实验功能 (Settings → Experimental features), or turn off 本地整句模型 (Local sentence model) under 设置 → 候选 (Settings → Candidates); ranking goes back to the dictionary only. This doesn't affect AI services.
To use your own model, see [Data files and logs](../advanced/data-files.md#Using-your-own-local-sentence-model).

### Experimental features

On macOS and Windows, 设置 → 实验功能 (Settings → Experimental features) lets you download two local sentence models. Both have to be downloaded by hand:

- **Zhiyi** (recommended): reads the characters before the cursor and the pinyin you typed, and picks the first candidate from the top few in the candidate window. It's trained and licensed by the author of [Zhiyi IME](https://github.com/dd1000001000/zhiyi_ime); about 154 MB on macOS and 70 MB on Windows. While it's on you can pick a level: 省电 (Power saving), 均衡 (Balanced) or 准确 (Accurate). Higher levels compare more candidates and read more context, and use more power; pick power saving on older computers or if typing feels slow or the machine gets hot.
- **Hanzhang Tongbian**: trained by Qingjian; writes whole sentences straight from the pinyin you typed, including words the dictionary doesn't have, so the first candidate for long sentences reads more naturally. It doesn't read the characters before the cursor; about 43 MB.

Click 下载 (Download) to see a progress bar; you can cancel. Once downloaded, the model is turned on. Only one model is used at a time: turning one on turns the other off, and with both off no model is used and ranking uses the dictionary only. Models are fetched from both Gitee and GitHub, from whichever answers first, switching if one drops, and every part is checked against its checksum. Linux doesn't support these two models yet.
