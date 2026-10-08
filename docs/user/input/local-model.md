---
title: 本地整句模型
order: 5
description: 随包的小模型在本机给整句候选重新排序：怎么生效、什么时候不生效、怎么关。
---

青简的本地模型系列叫「含章」。含章·通变（Hanzhang Tongbian）负责拼音解码与纠错；含章·知微（Hanzhang Zhiwei）负责根据上下文重排候选。优先使用通变，设备上没有它时使用知微。全程离线，不联网、不发送任何内容。

## 怎么生效

敲完一段拼音停一下（不到十分之一秒），整句候选就换成模型认为更通顺的那句；没停就按词库统计出的先显示。
通变根据这段拼音选句，不读取光标前文。回退到知微时，模型会参考光标前面的几十个字；应用不让读上下文时（部分终端、Electron 应用）只看这次会话里输入过的字，Linux 目前也只看本次会话的字。

模型会调整整句候选；词库拼不出完整句子时，还可能补出整句候选。词候选、你选过的词都不受它影响；你的习惯仍然优先。

macOS 14 等缺少所需 Metal 功能的系统会自动用 CPU 运行本地模型，加载和重排可能更慢。

## 什么时候不生效

- 已经翻到后面的页或用方向键移过高亮时，模型的结果不再换掉正在看的这页。
- 密码框里不组句；应用声明为私密的输入框里不读上下文。
- 输入法刚启动的几秒钟模型还在加载，这段时间按词库统计出句。

## 关掉

「偏好设置 → 云服务」（Windows：「设置 → 云服务」）关闭「本地整句模型」，或配置文件 `[model]` 里 `enabled = false`（Linux 只有这一种，改完重启青简服务）。关闭后只用词库统计，与云联想互不影响。

## 自己的模型

把 `hanzhang-tongbian-small.qjm` 放进用户数据目录的 `models/hanzhang-tongbian/`，重启输入法后优先使用。含章·知微使用 `hanzhang-zhiwei-small.qjm`，放在 `models/hanzhang-zhiwei/`，仅在没有通变时使用。旧用户目录 `model-p2c/` 与 `model/` 仍可读取；同一模型的新目录优先。直接导出的 `model.safetensors`、`config.json`、`vocab.json` 三个文件也能识别；同目录里两种格式都有时优先用 `.qjm`：

- **macOS**：`~/Library/Application Support/Qingjian/models/hanzhang-tongbian/`
- **Windows**：`%APPDATA%\Qingjian\models\hanzhang-tongbian\`
- **Linux**：`~/.local/share/qingjian/models/hanzhang-tongbian/`
