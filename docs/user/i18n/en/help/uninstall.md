---
title: Uninstalling
order: 4
description: "Uninstalling Qingjian, deleting it together with your learning data, or clearing only some of the data."
---

## macOS

Open Terminal and run:

```sh
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh
```

This removes only the input method itself; learning data and settings are kept and can be used again after reinstalling. To delete the data too:

```sh
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh --purge
```

This deletes everything in Qingjian's data folder, including imported dictionaries, learned words, the vocabulary book, the input log, settings and keys.

## Windows

Find 青简 (Qingjian) under Settings → Apps → Installed apps and uninstall it, or use 卸载青简 (Uninstall Qingjian) in the Start menu.
Programs that are open need to be closed before everything can be removed; the uninstaller asks whether to restart.

Learning data and settings are kept and can be used again after reinstalling; to delete the data too, delete Qingjian's data folder by hand after uninstalling. For its location, see [Data files and logs](../advanced/data-files.md#Data-files).

## Linux and Android

For Linux, see [Linux](../platforms/linux.md#Updating-and-uninstalling). On Android, uninstall the 青简 (Qingjian) app in the system settings; uninstalling also deletes the learning data and keys on the phone.

All data exists only on your device; once deleted it can't be recovered, and there are no cloud copies.

## Clearing only some data

- To clear only the input log: 设置 → 高级 → 清空输入日志 (Settings → Advanced → Clear input log).
- To clear the vocabulary book for one language: 设置 → 生词本 → 清空 (Settings → Vocabulary book → Clear).
- To delete a word Qingjian has learned: right-click that candidate while typing pinyin and choose 删除 (Delete); see [Typing habits](../personalize/habits.md#Candidate-right-click-menu).
- To remove an imported dictionary: 移除 (Remove) in 设置 → 词库 (Settings → Dictionaries) only moves the file to `dicts/removed/` in the data folder; delete that folder yourself to remove it for good.
