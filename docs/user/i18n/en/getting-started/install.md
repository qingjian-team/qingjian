---
title: Installation
order: 1
description: "Installing on macOS and Windows: system requirements, installers, what to do when the system blocks the first launch, and upgrading. Linux and Android have their own pages."
---

The macOS and Windows versions are in beta. Get the installer from the [download page](https://qingjian.app/en/download).
The Linux version is installed from a terminal, see [Linux](../platforms/linux.md); the Android version is installed from an APK, see [Android](../platforms/android.md).

The only official release channels for now are the website's download page, [GitHub Releases](https://github.com/qingjian-team/qingjian/releases), and the Quark Drive link given in each version's changelog; a "Qingjian" from anywhere else isn't an official release. Qingjian isn't published in any package manager yet.

## macOS

**Requirements**: macOS 13 or later. There is one installer for Apple Silicon and one for Intel; if you're not sure which Mac you have, choose  → About This Mac and look at the "Chip" line.

1. Download the installer with `arm64` in its name (Apple Silicon) or `x86_64` (Intel), double-click it and install; you'll be asked for an administrator password.
2. Beta builds aren't signed with an Apple developer ID yet, so the system blocks the first launch: go to System Settings → Privacy & Security, click "Open Anyway" at the bottom, and install again.
3. When installation finishes, "青简" (Qingjian) appears in the input menu.

If it doesn't appear, log out and back in (the system refreshes the list of input methods only at login). If it still doesn't appear, go to System Settings → Keyboard → Input Sources → Edit → + and add "青简" under "Chinese, Simplified".

After installing:

- A 中 / 英 (Chinese / English) item appears at the top right of the menu bar; click it to open the input method menu and settings.
- Before you start, choose a learning language under 设置 → 候选 → 译文 (Settings → Candidates → Gloss). The gloss beside each candidate will be in that language; see [Glosses beside candidates](../learning/translation.md).

## Windows

**Requirements**: 64-bit Windows 10 (version 1809 or later) or Windows 11. The installer includes everything it needs; nothing else has to be installed.

1. Download the installer whose name ends in `setup.exe`, double-click it and install; administrator rights are required.
   Keep the default location (`C:\Program Files\Qingjian`) and don't move it to another folder or drive: if it's installed elsewhere, the candidate window gets hidden behind system UI such as the Start menu and the taskbar search box.
2. Beta builds don't have a proper code signature yet, so SmartScreen blocks them: click "More info → Run anyway".
3. When installation finishes, Qingjian appears in the list of input methods (the 中 / 英 or 拼 indicator at the bottom right of the taskbar, or press `Win + Space` to switch), under "Chinese (Simplified)".

After installing:

- The taskbar shows whether you're currently in 中 (Chinese) or 英 (English); you can also open a floating status bar on the desktop, see [Chinese and English](../input/english-mode.md#Floating-status-bar-(Windows)).
- Settings are in "青简设置" (Qingjian Settings) in the Start menu, or the ⚙ on the floating status bar.
- Before you start, choose a learning language under 设置 → 候选 → 译文 (Settings → Candidates → Gloss).

If you can't switch to Qingjian in some program, log out and back in, or restart that program.

**Known issue**: because beta builds don't have a proper code signature, the candidate window may be hidden and invisible in system UI such as the Start menu, the taskbar search box and Settings; typing and committing text still work. This will be fixed in the signed release.

## Upgrading

Just install the new installer; your learning data and settings are kept.

On Windows, **programs that were already open before the upgrade only pick up the new version after you reopen them**: switching away from the input method and back doesn't help, and the taskbar search box only uses the new version after File Explorer restarts.
Changing settings isn't affected; changes take effect within a second.

To uninstall, see [Uninstalling](../help/uninstall.md).
