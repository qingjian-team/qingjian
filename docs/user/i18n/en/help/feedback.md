---
title: Reporting problems
order: 3
description: "Where to report problems and suggestions, and which logs and information to include."
---

Please report problems and suggestions in [GitHub Issues](https://github.com/qingjian-team/qingjian/issues/new/choose) by filling in the form; the form asks for the material below. Reports in English are welcome.
You can also join the QQ group [902314603](https://qm.qq.com/q/jBvn2gGTxm) (the Qingjian beta user group, in Chinese) to talk directly.

Check the [FAQ](faq.md) first; many situations are covered there.

## Please include

1. **A log bundle**: 设置 → 高级 → 日志文件 → 打包到桌面 (Settings → Advanced → Log files → Bundle to desktop) packs the logs, settings and system information (system version, architecture, locale and so on) into one file on your desktop; attach it to your report. API keys are scrubbed.
   You can also click 打开日志目录 (Open log folder) in the same place and take the files yourself; for the location, see [Data files and logs](../advanced/data-files.md#Runtime-logs).
   On Android, use 数据 → 日志 → 打包日志 (Data → Logs → Bundle logs) in the Qingjian app, then pick 发送给… (Send to…) to send it straight to QQ, WeChat or email, or 存到手机 (Save to phone) to keep the zip; it also includes the phone model, system version, battery settings and enabled input methods.
2. The app you were in when the problem occurred, what you typed, what you expected and what actually happened.
3. Your system version and Qingjian's version (设置 → 关于 (Settings → About)); not needed if you attach a log bundle.

If you can reproduce it, turn on 详细日志 (Verbose logging) (设置 → 高级 (Settings → Advanced) on a computer, 数据 → 日志 (Data → Logs) on Android) and reproduce it once more, so the log contains a key-by-key record; please turn it off afterwards, because it records what you type.
