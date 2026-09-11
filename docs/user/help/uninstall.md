---
title: 卸载
order: 2
description: 卸载青简、连同学习数据一起删除、只清除输入日志。
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

此命令删除「~/Library/Application Support/Qingjian/」中的全部内容，包括导入的词库、学习到的词、输入日志、配置与密钥。

## Windows

在「设置 → 应用 → 安装的应用」中找到「青简」并卸载，或使用开始菜单的「卸载青简」。
正在运行的应用中的输入法组件需等这些应用关闭后才能完全清除，卸载程序会提示是否重启。

学习数据与设置保留在「%APPDATA%\Qingjian」，重新安装后仍可用；连同数据一起删除时，卸载后手动删除该文件夹。

所有数据均为本机文件，删除后不可恢复，没有云端副本。

## 只清除一部分

- 只清除输入日志：「偏好设置 → 高级 → 清空输入日志」。
- 删除某个学到的词：输入拼音时按 `⇧ + 数字`，见 [译词与生词](../learning/translation.md#删除不需要的候选)。
- 移除导入的词库：「偏好设置 → 词库 → 移除」只将文件移到数据目录的 `dicts/removed/`，需彻底删除时自行清理该目录。
