---
title: Pinyin schemes and fuzzy pinyin
order: 5
description: "Choosing full pinyin, one of nine double pinyin schemes or Dachen Zhuyin; turning on the nine fuzzy pinyin pairs and how they're ranked; how the double pinyin schemes differ, and key hints; keys for Dachen Zhuyin."
---

## Choosing a scheme

设置 → 输入 (Settings → Input) has two independent settings:

| Setting | Options |
| --- | --- |
| 拼音 (Pinyin) | Full pinyin (default), Xiaohe, Ziranma, Microsoft, Sogou, Zhineng ABC, Xiaolang, Shoudao, Pinyin Jiajia and Ziguang double pinyin, plus [double pinyin schemes you make yourself](../advanced/custom-schemes.md#Custom-double-pinyin-schemes); Dachen Zhuyin; Off |
| 形码 (Shape-based) | Off (default), Wubi (86 edition), plus shape-based schemes you make yourself; see [Wubi and mixed input](wubi.md) |

Each can be on by itself, or both can be on at once (that's mixed input). They can't both be off: when one is off, the other's Off option is greyed out.
Full pinyin, double pinyin and Zhuyin share the same dictionary, sentence conversion and typing habits, so what Qingjian has learned still applies after you change schemes. The Chinese/English switch doesn't depend on the scheme.

## A few spellings in full pinyin

- `lue` / `nue` are equivalent to `lve` / `nve`; `v` after j / q / x / y is accepted too (`tiqv` → 提取), including with a final (`xvexi` → 学习, `jvan` → 卷, `yvn` → 云).
- 嗯 is typed as `en` by convention (`enen` → 嗯嗯); `ng` on its own also gives 嗯, but use `en` when it's joined to other pinyin.
- `lo` shares its key sequence with `luo` and is treated as luo.

## Fuzzy pinyin

Nine pairs: z/zh, c/ch, s/sh, n/l, f/h, l/r, an/ang, en/eng, in/ing. All are off by default.

Tick them under 设置 → 输入 → 模糊音 (Settings → Input → Fuzzy pinyin); on macOS you can also tick them in the 模糊音 (Fuzzy pinyin) submenu of the input method menu. With z/zh on, `zi` also gives 知 / 之; with f/h on, `kaiha` also gives 开发.
Words matching what you actually typed still come first, followed by fuzzy matches. The three final pairs only apply to complete syllables. Not available when Pinyin is set to Off.

## Double pinyin

Nine schemes: Xiaohe, Ziranma, Microsoft, Sogou, Zhineng ABC, Xiaolang, Shoudao, Pinyin Jiajia, Ziguang.

- The pinyin line in the candidate window shows the decoded full pinyin (`nihc` → `ni'hao`). With 输入框显示按键 (Show keys in the input box) on under 设置 → 输入 (Settings → Input), the app's input box shows the keys you typed (`kdfa`) instead, while the candidate window still shows full pinyin (`kai'fa`).
- Double pinyin supports fuzzy pinyin too. With s/sh on, `sd` in Ziranma / Microsoft / Sogou and `sl` in Xiaohe find 双.
- When the whole input can't form a single complete syllable, it's read again as initials, one per key: Pinyin Jiajia `im` → 什么, `zm` → 怎么. Input that decodes normally is unaffected, and complete double pinyin codes still work as before.
- Spelling mistakes aren't corrected in double pinyin.
- In double pinyin, v / u / i are all syllable keys, so calculations and asking about characters start with `Shift + V` and `Shift + U` instead; see [Dates, calculations and special characters](shortcuts.md#Changing-the-prefix-letters).

How the schemes differ:

- **Zero initial**: Xiaohe / Ziranma write the `aa` `ai` `ah` family (a, ai, ang); Microsoft / Sogou write `o` + the final key (`oa` `ol` `oh`); Zhineng ABC also writes `o` + the final key, but using its own final table (ei is `oq`, not `oz`), and doesn't accept doubled vowels (`aa` is zha, `ee` is che); Ziguang likewise only writes `o` + the final key (`oa` `or` `oj`) and accepts neither doubled vowels nor doubled finals; Xiaolang writes `aa` `ai` `ah` `ao`, `uu` (e), `ui` (ei), `un` (en/eng), `ur` (er), `oo` (o), `ou` (ou).
- **The `;` key**: in Microsoft / Sogou / Ziguang, `;` is ing: when an initial is left on its own at the end of the pinyin, `;` acts as pinyin (`x;` → xing); otherwise it's still punctuation.
- **ü**: in Microsoft / Sogou the ü key is `y`: `ny` / `ly` give 女 / 绿, and `jy` / `qy` / `xy` / `yy` decode to ju / qu / xu / yu; the usual `ju` / `qu` / `xu` / `yu` still work. Other schemes also accept their ü key combined with j / q / x / y, but an existing reading of the same keys takes priority (Microsoft `jv` is still jue, Xiaolang `jv` is still jing).
- **Zhineng ABC**: the retroflex initials are `a` for zh, `e` for ch and `v` for sh (`ai` → zhi, `ei` → chi, `vi` → shi).
- **Xiaolang**: the retroflex initials are `e` for zh, `i` for ch and `v` for sh; `x` is ü (`lx` → lv, `nx` → nv, `jx` → ju); `v` doubles as the finals uai and ing; finals not in the key table are typed with their full-pinyin keys. `en` and `eng` share a code (`un`) and are treated as en; `dia` / `dai` share a code and are treated as dai, `lia` / `lai` as lai, and `nen` / `niang` as niang.
- **Shoudao**: the retroflex initials are `v` for zh, `i` for ch and `e` for sh; zero-initial syllables double the first letter or follow full pinyin (`aa` `ai` `an` `ao` `oo` `ou` `en` `er`), ang is `ay`, and e / ei / eng are led by `u` (`ue` `ui` `uf`); ue is on `l` (`jl` → jue) and üe has its own key `b` (`lb` → lve). `den` / `dia` share a code (`dk`) and are treated as den.
- **Pinyin Jiajia**: the retroflex initials are `v` for zh, `u` for ch and `i` for sh (ch / sh are the reverse of Xiaohe and Ziranma); frequent finals sit on the home row: ai→S, ao→D, an→F, ang→G, ei→W, en→R, eng→T, with ing on `q`; ü is on `v` (`nv` 女, `lv` 绿) and üe on `x` (`jx` → jue, `nx` → nüe); zero-initial syllables are mainly doubled (`aa` `as` `af`, `ee` `er` `eq`, `oo` `op`; note that en is `er` and er is `eq`), with the o prefix also accepted (`oa` `oe`); the `;` key isn't used.
- **Ziguang**: the retroflex initials are `u` for zh, `a` for ch and `i` for sh (`ui` → zhi, `ai` → chi, `ii` → shi; `a` and `i` double as finals); `;` is ing; ü is on `v` (`nv` 女, `lv` 绿), and üe shares `n` with ue and ui (`jn` → jue, `yn` → yue, `gn` → gui); zero-initial syllables are only `o` + the final key, with neither doubled vowels (`aa` is cha) nor doubled finals `ii` `uu` (those are shi and zhu).

If a layout doesn't suit your hands, or the scheme you want isn't listed, you can make your own; see [Custom schemes and code tables](../advanced/custom-schemes.md#Custom-double-pinyin-schemes) (macOS, Windows).

### Key hints

If you can't remember the layout, the candidate window can show a keyboard for the current scheme next to it: each key is labelled with its final, and retroflex initials (zh / ch / sh) that occupy a key are shown at its top right. After you type an initial, keys that complete a syllable turn green and show that syllable; keys that can't are dimmed.

Choose under 键位提示 (Key hints) in 设置 → 输入 (Settings → Input):

- **卡住时显示 (When stuck)**: appears only when you pause partway through or type a key that can't form a syllable, so you won't see it while typing smoothly. Once it appears it stays for the rest of that input and closes after you commit.
- **一直显示 (Always)**: shown the whole time you're typing.

键位提示大小 (Key hint size) and 键位提示透明度 (Key hint opacity) set the keyboard's size and transparency. The keyboard is placed on the side of the candidate window facing away from the cursor so it doesn't cover the line you're typing; if it doesn't fit anywhere, it isn't shown that time.
Supported on macOS and Windows; on Windows, the candidate window must be drawn by Qingjian's renderer (the default). Wubi's radical chart uses the same settings; see [Wubi and mixed input](wubi.md#Radical-chart).

## Dachen Zhuyin

Supported on Windows and Linux (Android also has a Zhuyin keyboard), not yet on macOS. With 拼音 (Pinyin) set to 大千注音 (Dachen Zhuyin) under 设置 → 输入 (Settings → Input), you type Zhuyin symbols on the Dachen keyboard layout, and candidates share the same dictionary as full pinyin. While typing, the keys become:

| Action | Keys |
|---|---|
| Commit the highlighted candidate | `Enter` |
| Commit the Zhuyin symbols as typed | `Shift + Enter` |
| First tone / neutral tone | `Space` |
| Tones and symbols (keep typing, don't pick) | Number keys, `-` `;` `,` `.` `/` |

The Dachen keyboard assigns the number keys and `- ; , . /` to Zhuyin symbols and tones, so with Zhuyin these keys no longer pick words; use `Enter` to pick, and `Space` is the first tone.
For a syllable typed with a tone, words with that tone come first (ㄇㄚ with the first tone gives 妈 first, ㄇㄚˇ gives 马 first), and sentences are converted by tone too (ㄨㄛˇ ㄒㄧㄤˇ ㄇㄞˋ gives 我想卖); without a tone there's no restriction.
On Linux you can also press `Space` after the tone to pick.
