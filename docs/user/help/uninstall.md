---
title: 卸载
order: 2
description: 怎么卸载青简、怎么连学习数据一起删干净、怎么只清输入日志。
---

## 卸载输入法

打开「终端」，运行：

```sh
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh
```

这只删输入法本身，学习数据与设置保留，以后重装还在。

## 连数据一起删

```sh
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh --purge
```

会把「~/Library/Application Support/Qingjian/」里的一切删掉，包括你导入的词库、学习到的词、输入日志、配置与密钥。所有数据都是本机文件，删了就没了，没有云端副本。

## 只清一部分

- 只清输入日志：「偏好设置 → 高级 → 清空输入日志」。
- 删某个学到的词：候选窗口里 `⇧ + 数字`，见 [译词与生词](../learning/translation.md#快捷键)。
- 移除导入的词库：「偏好设置 → 词库 → 移除」只是把文件挪到数据目录的 `dicts/removed/`，要真删自己清那个目录。
