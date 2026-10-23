---
title: Chinese and English
order: 2
description: "Entering English mode, changing or turning off the Chinese/English switch, default mode and per-app memory, English completion and correction, typing English in Chinese mode, automatic spacing, and the floating status bar on Windows."
---

## Entering English mode

- **macOS**: English mode is on while the `Caps Lock` light is on; turn it off to return to Chinese.
  If you want `Caps Lock` to switch between Qingjian and "ABC" the way the system pinyin input does, turn on the option to switch to ABC with Caps Lock in System Settings → Keyboard → Text Input → Edit:
  a quick press of `Caps Lock` then switches input methods and holding it still locks capitals; Qingjian's own English mode is no longer needed.
- **Windows**: tap `Shift` (press and release on its own, without another key) to switch between Chinese and English. The taskbar shows 中 or 英 to match, and A while Caps Lock is on.
  Clicking the 中 / 英 icon on the taskbar also switches.
  On Windows, `Caps Lock` only controls case: while it's on, capital letters are typed directly in either mode (as with Microsoft Pinyin).
- **Linux**: tap `Shift` to switch between Chinese and English; holding `Shift` while typing letters doesn't switch. While `Caps Lock` is on, capital letters are typed directly.

On Windows, the Chinese/English mode is **shared by all apps** by default: switch to English in one app and other apps, including newly opened ones, are in English too. On Linux, **each window keeps its own**, and new windows start in Chinese.

## Changing or turning off the Chinese / English switch

Hitting `Shift` by accident while typing switches to English and interrupts your typing. On Windows and Linux you can adjust this under 切换键 (Switch key) in 设置 → 中英文 (Settings → Chinese/English); every option you tick works, and you can tick several:

| Option | Notes |
|---|---|
| 单击 Shift (Tap Shift) | Ticked by default, as in Microsoft Pinyin |
| 单击 Ctrl (Tap Ctrl) | Hard to trigger by accident; Ctrl's role in other key combinations is unaffected |
| Ctrl + Alt + Space | A key combination, the hardest to trigger by accident |
| Ctrl + Space (system, Windows only) | Windows' built-in "IME / non-IME toggle", on by default; this changes a Windows system setting and also affects Microsoft Pinyin and other Simplified Chinese input methods |

Changes take effect immediately. With nothing ticked, the keyboard no longer switches between Chinese and English: on Windows, only the 中 / 英 buttons on the taskbar or floating status bar do; on Linux, a tap of `Shift` is passed to the app unchanged.

On Windows, if you also have an English keyboard or another input method installed, the system may use `Ctrl + Space` to switch to it; that's the system's decision. If you don't want this system hotkey, go to Windows Settings → Time & language → Typing → Advanced keyboard settings → Input language hot keys and set it to "Not assigned".

On Linux, Fcitx5 by default sets **left** `Shift` to "temporarily switch between the current and the first input method", and it receives the key before Qingjian does: tapping left `Shift` switches straight to the English keyboard regardless of Qingjian's switch key setting.
To let only Qingjian handle left `Shift`, clear that entry under Global Options → Hotkey in the Fcitx5 configuration.

If you prefer to use two separate input methods for Chinese and English, turn off **英文模式 (English mode)** in the first row of 设置 → 中英文 (Settings → Chinese/English): Qingjian then stays in Chinese, and neither the switch key nor the status bar button switches to English.
When you need English, switch to another input method with `Win + Space` on Windows or Fcitx's switch hotkey on Linux.

## Default mode and per-app memory

The 英文 (English) group in 设置 → 中英文 (Settings → Chinese/English) has two options:

- **默认英文模式 (Start in English mode)** (Windows, Linux): on Windows, Qingjian starts in English; on Linux, newly opened apps and windows start in English. When off, they start in Chinese.
- **按应用记住中英模式 (Remember mode per app)** (Windows): switching back to an app restores the mode it had last time. Apps with no record use the default mode above, and all windows of one app share a mode.

For example, with both on, new apps start in English, and switching Notepad to Chinese doesn't change the browser's English mode.
Remembered modes last for as long as Qingjian is running, so closing and reopening an app still restores them; after Qingjian restarts, the default mode applies again.
Changing either option or toggling 英文模式 (English mode) clears the remembered modes. When English mode is off, it's always Chinese and "Start in English mode" has no effect.

## Typing in English mode

In English mode, the candidate window offers English word completions and spelling corrections as you type: `recieve` → receive, `seperate` → separate.
Letters are lowercase by default; hold `Shift` for capitals.

- Picking works as in Chinese mode: `Space` picks the highlighted candidate (and the space is still typed after the word), a number key picks candidate N on this page (if there's no candidate N, the digit joins the word, as in `foo1`), paging keys page, `↑` / `↓` move the highlight, and `Tab` also picks the highlighted item.
- **`Enter` and punctuation commit the letters exactly as typed**: words that aren't in the word list, like `kubectl`, come out as typed when you press `Enter`; with no candidates, `Space` also commits as typed.
- For words with digits (`mp3`, `utf8`), press `Enter` to output the letters, then type the digits; with no candidates, digits simply follow the letters.
- Punctuation is always half-width on macOS; on Windows and Linux it's half-width by default and can be made full-width, see [Punctuation, symbols and emoji](punctuation.md).

English words you've committed as typed with `Enter` (like `gist` or `python`) are remembered and come first next time. English candidates you've picked are remembered too.

### Which apps show candidates

- By default, English candidates appear only in browsers, chat and document apps, such as Safari, Chrome, Edge, Firefox, WeChat, QQ, WeCom, DingTalk, Feishu, Word, WPS, Notepad, Notes and Obsidian.
  In other apps, English mode shows no candidates and letters go straight to the app: in terminals and code editors it won't cover the app's own completion or take over `Tab`,
  and single-key shortcuts in software like Photoshop, Illustrator and Blender (such as Photoshop's `W` `H` `V`) work directly.
- 只在写字的应用里给 (Only in writing apps) in 设置 → 中英文 (Settings → Chinese/English) controls this list; untick it to get candidates everywhere. To add or remove specific apps, see [Editing the configuration file](../advanced/config-file.md).
- Turning off 英文模式也给候选 (Candidates in English mode) means English mode shows no candidates at all, just as if no input method were in use.

## Typing English in Chinese mode

You don't need to switch in Chinese mode:

- When the whole input is an English word, English candidates appear directly. If it isn't valid pinyin (`hello`), English comes first; if it is (`china`), English comes second.
  With 中文排在英文词前面 (Chinese before English words) on under 设置 → 候选 (Settings → Candidates), English always comes second (`hello` gives 荷兰咯 first, then hello), so `Space` still commits Chinese and `2` picks the English word; when there's no Chinese candidate (`github`), English still comes first.
  Proper nouns are committed with the correct capitalization: `windows` → **Windows**, `github` → **GitHub**, `python` → **Python**.
- When the input isn't valid pinyin and has at least 3 letters, English is completed: `compa` → company / companies / compared.
- For input like `key` or `id`, which looks like English but can also be read as pinyin, the Chinese word comes first if you usually pick it and the English word moves down.
- When a sentence of pinyin ends with an English word, the sentence includes that word: `woxiangxuehaorust` → 我想学好rust, `wodeid` → 我的ID, `yongwindows` → 用Windows.
  The English word has to be in the word list (words you've typed in English mode count). Input like `database`, which can also be read as pinyin, is ranked by how natural each reading is; short words like `fan` and `to` are treated as pinyin.
  English words are only recognized at the end of a sentence; words in the middle, as in `wozaidebugzhege`, aren't supported yet (commit the Chinese before it first, then type the English word).
- **Words made of a letter plus Chinese characters** are typed as "letter + pinyin": `cpan` → C盘, `bzhan` → B站, `txu` → T恤, `agu` → A股, `xguang` → X光.
  Since `u` is the prefix for asking about characters, type U盘 as `youpan` (same sound as 优盘; U盘 comes first). These words also work in sentences: `wozaibzhan` → 我在B站.
- If you don't want English words in Chinese mode, turn off 打拼音时也给英文词 (English words while typing pinyin) under 设置 → 候选 (Settings → Candidates); English mode isn't affected.

By default, `Shift + letter` while typing pinyin types English temporarily: the pinyin is committed first and the capital letter is output as typed.
If you change Shift + 字母 (Shift + letter) in 设置 → 中英文 (Settings → Chinese/English) to 进组句 (Join the input), the capital letter takes part in the pinyin as lowercase, so `Cpan` gives C盘 just like `cpan`.

Typing a symbol like `-` `/` `@` switches to direct English input, so you can type strings with symbols such as `no-way`; see [Punctuation, symbols and emoji](punctuation.md#Typing-punctuation-while-typing-pinyin).

## Automatic spacing between Chinese and Latin text

On macOS and Windows you can turn on 中文与英文、数字之间自动加空格 (Add spaces between Chinese and English/digits) under 设置 → 中英文 (Settings → Chinese/English) (off by default). When on, a space is added automatically wherever Chinese characters meet half-width letters or digits:
用Go写了30行 is committed as 用 Go 写了 30 行. No space is added next to punctuation or an existing space, and custom phrases and candidates such as dates and calculations are committed as is.
If the cursor has moved elsewhere, or the app doesn't provide the text before the cursor, the preceding text can't be checked and no space is added. Not yet supported on Linux.

## Floating status bar (Windows)

If the 中 / 英 indicator on the taskbar is easy to miss, you can open a floating status bar on the desktop: turn on 悬浮状态条 (Floating status bar) in 设置 → 中英文 (Settings → Chinese/English). It has four cells:

| Cell | Shows | Click to |
| --- | --- | --- |
| `中` / `注` / `英` | The current mode (注 for Dachen Zhuyin; double pinyin and Wubi show something like 中 · 小鹤双拼) | Switch Chinese / English; always 中 when English mode is off |
| `，。` / `,.` | Whether punctuation is full-width or half-width in the current mode; Chinese and English each keep their own | Switch full-width / half-width for the current mode |
| `符` | | Open the [symbol panel](punctuation.md#Symbol-panel) |
| ⚙ | | Open Settings |

Drag it anywhere on the screen; its position is remembered. Its look follows the system's light or dark appearance. Adjust its size under 状态条大小 (Status bar size) (60%–150%); it doesn't follow the candidate font size.
It only appears while Qingjian is the current input method: it hides when you switch to another input method or the system's English keyboard and comes back when you switch back.
It hides automatically while a full-screen app (a game, full-screen video, a presentation) is in front, and returns when you leave full screen. You can also show or hide it at any time from the right-click menu of the 中 / 英 icon on the taskbar.
Each time it appears, it's moved back into the visible area of the screen. The status bar can also change its look with the theme; see [Custom themes](../advanced/custom-themes.md#Floating-status-bar-(Windows)).
