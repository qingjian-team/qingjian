---
title: FAQ
order: 2
description: "What to do when Qingjian is missing after installing, the candidate window is hidden, nothing changes after an upgrade, typing does nothing, or you can't type in certain apps."
---

## Installing and upgrading

**Qingjian isn't in the input method list after installing**
On macOS, log out and back in first; if it's still missing, add it under Chinese, Simplified in System Settings → Keyboard → Input Sources → Edit → +. On Windows, log out and back in.
If it's still missing after a restart, please [report it](feedback.md) with your system version and how you installed it.

**The current input method isn't Qingjian after logging in** (macOS)
A known issue; switch to it by hand once. It will be fixed in the signed release.

**Nothing changed after installing a new version** (Windows)
Programs that were already open before the upgrade only pick up the new version after you reopen them; switching away from the input method and back inside them doesn't help, and the taskbar search box only uses the new version after File Explorer restarts. Changing settings isn't affected.

## The candidate window

**The candidate window is hidden behind the Start menu, the taskbar search box or Settings** (Windows)
Two things cause this; typing and committing text still work:

- Qingjian isn't installed in the default location (`C:\Program Files`). Uninstall it and reinstall to the default location.
- Beta builds don't have a proper code signature yet; this will be fixed in the signed release.

**The candidate window covers a terminal's or editor's own completion**
By default, English mode only shows candidates in browsers, chat and document apps, not in these apps; if it's Chinese candidates in the way, switch to English mode. See [Chinese and English](../input/english-mode.md#Which-apps-show-candidates).

**Pinyin is displayed oddly in some app**
Try changing 拼音显示位置 (Pinyin position) under 设置 → 外观 (Settings → Appearance), for example to 只在窗口 (In the window only), and please [report](feedback.md) the app name and what happens to the author.

**Candidates in full-screen games** (Windows)
In games that draw input method candidates themselves, the candidates appear in the game's own interface and Qingjian's candidate window stays hidden. In an exclusive full-screen game that doesn't draw them, Qingjian's candidate window is covered by the game; typing and committing still work.

**The floating status bar has disappeared** (Windows)
It's only shown while Qingjian is the current input method, and it hides in full-screen apps. Check that 悬浮状态条 (Floating status bar) is on under 设置 → 中英文 (Settings → Chinese/English), or show it from the right-click menu of the 中 / 英 icon on the taskbar.

## Typing

**Nothing appears when typing: no pinyin and no candidate window** (Windows)
Qingjian's background program may not be running. The input method restarts it on its own when it notices, and the next key works again; if there's still no response, start 青简 Server (Qingjian Server) from the Start menu, or log out and back in.

**Can't switch to Qingjian, or typing does nothing, in some app** (Windows)
Log out and back in, or restart that app.

**Can't type in security software such as Tencent Software Manager or PC Manager** (Windows)
Such software blocks third-party input methods from its own windows, so neither Qingjian nor other third-party input methods (except the built-in Microsoft Pinyin) can type or switch Chinese/English in its search box. This is the software's security mechanism and can't be fixed from the input method side.

**Ctrl + Space switched to another input method** (Windows)
`Ctrl + Space` is Windows' "IME / non-IME toggle". With only Qingjian installed it switches Chinese/English; with an English keyboard or another input method installed, Windows may use it to switch input methods instead, which is the system's decision. For what to do, see [Chinese and English](../input/english-mode.md#Changing-or-turning-off-the-Chinese-English-switch).

**Single-key shortcuts in Photoshop and similar software (`W` `H` `V`) don't work**
Switch to English mode; in English mode, letters go to such apps as typed.

**Some keys are wrong with a non-US keyboard layout** (Windows)
Digits and common punctuation on the main keyboard are recognized according to the current system keyboard layout; letters, AltGr and dead keys on non-US layouts still have limitations.

**Keys are swallowed in some app, or translating selected text can't read the text**
Please [report](feedback.md) the app name and what happens to the author.

## Settings

**The settings window won't open, or a setting doesn't take effect**
Use the legacy settings instead, see [The settings window](../getting-started/settings.md#Legacy-settings), and please report it.

**I made a mistake editing the configuration file by hand**
Qingjian keeps using the last valid settings, and the macOS input method menu shows an extra grey line describing the error; fix it as described and save. See [Editing the configuration file](../advanced/config-file.md).
