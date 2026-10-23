---
title: Dates, calculations and special characters
order: 4
description: "Typing dates, times and weekdays with rq / sj / xq; calculating, writing Chinese numerals and amounts, and Chinese dates with a v prefix; typing characters by code point with a u prefix."
---

## Dates, times and weekdays

| Input | Result |
|---|---|
| `rq` / `sj` / `xq` | Today's date (`2026年9月3日`, `2026-09-03`), the current time (`19:06`, `19点06分`), the day of the week, inserted after the first candidate |
| `riqi` / `shijian` / `xingqi` | When 日期 (date), 时间 (time) or 星期 (weekday) appears among the candidates, the same forms follow right after it |

## Calculations, Chinese numerals and dates

Start with `v`:

| Input | Result |
|---|---|
| `v1+2` | Calculation: gives `3` and `1+2=3`; supports `+ - * / ^`, parentheses and decimals |
| `v123` | Chinese numerals: 一百二十三 and 壹佰贰拾叁; amounts: 一百二十三元整 and 壹佰贰拾叁元整 |
| `v123.5` | Decimal reading: 一百二十三点五 and 壹佰贰拾叁点伍; amounts: 一百二十三元五角 and 壹佰贰拾叁元伍角 (up to two decimal places, i.e. fen) |
| `v1998-3-8` | Chinese date: 一九九八年三月八日; plus 1998年3月8日 and 1998-03-08. The separator can be `-`, `/` or `.` |

- A date is recognized as **a four-digit year + month + day**; if the year isn't four digits (`v98-3-8`) or the month and day aren't valid (`v1998-2-30`), it's treated as a calculation.
- After a `v`, digits and operators all go into the expression and are no longer used to pick or page; to pick the second item, use `↑` / `↓` and `Space`. `Enter` outputs what you typed as is.
- English words starting with `v` (`very`) aren't affected and still get English candidates.

## Typing characters by code point

`u` followed by a hexadecimal character code (a Unicode code point) gives that character: `u4e00` gives 一, `u+1f600` gives 😀.

`u` followed by pinyin asks about a character, which requires AI services; see [Asking and translating](../cloud/question-and-translate.md#Asking-about-a-character).

## Changing the prefix letters

`v` and `u` can be changed under 表达式 (Expression) and 问字 (Ask about a character) in 设置 → 快捷键 (Settings → Shortcuts). Only v, u and i are allowed (no pinyin syllable starts with them); picking a letter that's already in use swaps the two.

With double pinyin and Wubi–pinyin mixed input, v / u / i are all code keys, so hold `Shift` instead: `Shift + V` followed by a calculation (`V1+2`), `Shift + U` followed by pinyin or a code point (`U4e00`), as in Sogou and Microsoft double pinyin. This only works before you've started typing pinyin; if you've changed the prefix letters, use the new letter with `Shift`.
Wubi on its own has none of these prefixes.
