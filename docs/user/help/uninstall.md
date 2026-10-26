---
title: 卸载
order: 4
description: 卸载青简、连同学习数据一起删除，或只清除一部分数据。
---

## macOS

打开「终端」，执行：

```sh
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh
```

以上只删除输入法本身，学习数据与设置保留，重新安装后仍可用。连同数据一起删除：

```sh
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh --purge
```

这会删除青简数据文件夹中的全部内容，包括导入的词库、学习到的词、生词本、输入日志、设置与密钥。

## Windows

在「设置 → 应用 → 安装的应用」中找到「青简」并卸载，或使用开始菜单的「卸载青简」。
正开着的程序要关闭后才能完全清除，卸载程序会提示是否重启。

学习数据与设置会保留，重新安装后仍可用；要连同数据一起删除，卸载后手动删除青简的数据文件夹，位置见 [数据文件与日志](../advanced/data-files.md#数据文件)。

## Linux 与 Android

Linux 见 [Linux](../platforms/linux.md#更新与卸载)。Android 在系统设置中卸载「青简」应用；卸载会一并删除手机上的学习数据与密钥。

所有数据都只在本机，删除后不可恢复，没有云端副本。

## 只清除一部分

- 只清除输入日志：「设置 → 高级 → 清空输入日志」。
- 清空某门语言的生词本：「设置 → 生词本 → 清空」。
- 删除某个学到的词：打拼音时右键那个候选，选「删除」，见 [输入习惯](../personalize/habits.md#候选右键菜单)。
- 移除导入的词库：「设置 → 词库」里的「移除」只把文件挪到数据文件夹的 `dicts/removed/` 里，需彻底删除时自行删除该文件夹。
