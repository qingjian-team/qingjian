---
title: 数据文件与日志
order: 2
description: 青简数据文件夹的位置、每个文件记的是什么、搬到另一台电脑、运行日志的位置，以及换用自己的本地整句模型。
---

青简记下的东西概括在 [隐私](../help/privacy.md#本机记了什么)；本页列出具体的文件。

## 数据文件

数据文件夹的位置：

- **macOS**：`~/Library/Application Support/Qingjian/`（访达中按 `⇧⌘G` 输入此路径）。
- **Windows**：`%APPDATA%\Qingjian`（在资源管理器地址栏中直接输入此路径）。
- **Linux**：`~/.local/share/qingjian`；配置文件在 `~/.config/qingjian/`。设置了 `XDG_DATA_HOME` / `XDG_CONFIG_HOME` 时用它们下面的 `qingjian`。

「设置 → 高级」的「打开数据目录」直接打开它。文件都是纯文本，可随时打开查看：

| 文件 | 内容 | 含输入内容 |
|---|---|---|
| `config.toml` | 设置，见 [直接编辑配置文件](config-file.md) | 否 |
| `.env` | AI 服务的密钥，填写后才有；只有当前账户可读 | 否 |
| `user.tsv` | 每个词被选择的次数 | 选过的词 |
| `user-words.tsv` | 自己造的词与选过的 AI 候选 | 选过的词 |
| `user-choices.tsv` | 某段拼音下选过的词 | 拼音与选过的词 |
| `user-demoted.tsv` | 候选右键菜单里降权的词（哪段字母、哪个词、降了几次） | 拼音与词 |
| `user-days.tsv` 等 `*-days.tsv` | 上面几张表的最近使用日 | 否 |
| `user-english.tsv` | 原样上屏过的英文词 | 英文词 |
| `user-typos.tsv` | 被接受的拼写纠正（敲成了什么、本意是什么） | 拼音音节 |
| `user-ngram.tsv` | 词与词之间的接续 | 连续输入过的词 |
| `user-glossary-<语言>.tsv` | AI 补充的译词 | 词与译词 |
| `vocab-log.tsv` | 生词本的时间线：每次上屏有译词的中文一行（时刻、中文、译词、读音） | 中文与译文 |
| `user-vocab.tsv` | 生词本的词表：每条译词被看到 / 上屏 / 输出的次数，最近一次的中文与例句，是否标了认识 | 译词、中文与例句 |
| `usage-hourly.tsv` | 按小时的字数、词数、上屏次数、首选次数、键数、打字时长，只有数字 | 否 |
| `usage.tsv` | 0.2.2 以前按天的输入统计，只读不再写 | 否 |
| `api-usage.json`、`api-usage-hourly.tsv` | AI 服务的请求次数与 Token 数，只有数字 | 否 |
| `update.json` | 上次检查更新的时间与查到的新版本 | 否 |
| `input-log.jsonl` | 输入日志，见 [隐私](../help/privacy.md#输入日志) | 按键与上屏的文字 |
| `dicts/` | 导入的词库；移除的在 `dicts/removed/` | 否 |
| `codes/` | 导入的辅码码表 | 否 |
| `themes/` | 自制主题，见 [自制主题](custom-themes.md) | 否 |
| `schemes/` | 自制双拼、形码与辅码方案，见 [自制方案与码表](custom-schemes.md) | 否 |
| `models/` | 自己放入的本地整句模型，见下文 | 否 |

这些文件每次整体替换，即使输入法中途崩溃也不会留下写到一半的文件。

想让输入习惯从头开始，删掉 `user` 开头的这些表（`user-glossary-*`、`user-vocab.tsv` 除外）；生词本在设置的「生词本」页清空。

## 搬到另一台电脑

「设置 → 高级」的「导入、导出设置」把设置存成一个 `.toml` 文件，在另一台电脑上导入：

- 导出的文件不含自定义短语（在「自定义短语」页另行导入导出）、AI 服务的密钥和悬浮状态条在屏幕上的位置；导入时这几样保留这台电脑原来的。
- 导入会整份换掉其余设置，换之前把原来的配置另存为同目录的 `config.before-import.toml`，导错了可以拷回去。文件里的设置读不通时什么都不改。
- 输入习惯、自造词、导入的词库等数据文件不在里面；要一起搬，在两台电脑都完全退出青简后拷整个数据文件夹（同一版本之间）。

## 运行日志

- **macOS**：`~/Library/Logs/Qingjian/`，输入法为「qingjian.log.日期」，设置窗口为「qingjian-settings.日期.log」。
- **Windows**：`%LOCALAPPDATA%\Qingjian\logs`，后台程序（「server.日期.log」）、各应用里的输入法部分（「tsf.日期.log」）与设置窗口（「qingjian-settings.日期.log」）都在这一个文件夹。
- **Linux**：`~/.local/state/qingjian/logs`（设置了 `XDG_STATE_HOME` 时在它下面）。

「设置 → 高级 → 日志文件」的「打开日志目录」直接打开它；「打包到桌面」把日志连同设置（抹去密钥）打成一个 zip，反馈时附上，见 [反馈问题](../help/feedback.md)。
按天分文件，保留 7 天。缺省不记录输入的内容，只记在哪个应用里打字、每段敲了几个键、上屏了几个字这类计数；打开「详细日志」后才逐键记录。

Windows 后台的启动、受控退出及 Rust panic 位置都写入现有 `server.日期.log`，不另建日志文件。panic 正文不会写入。外部强制终止或原生异常仍可能来不及留下记录。
Windows 后台日志包含进程编号和启动时配置文件的实际路径。若从 Windows 打包应用中启动的工具显示设置已开启，而输入法未启用，应先核对实际路径：系统可能让工具读取应用私有的数据副本。

## 换用自己的本地整句模型

本地整句模型含章·通变在 macOS 与 Windows 上从「设置 → 实验功能」下载到下面的目录，Linux 版随包附带。要换用自己训练的模型，把 `hanzhang-tongbian-small.qjm` 放进数据文件夹的 `models/hanzhang-tongbian/`，重启输入法后优先使用：

- **macOS**：`~/Library/Application Support/Qingjian/models/hanzhang-tongbian/`
- **Windows**：`%APPDATA%\Qingjian\models\hanzhang-tongbian\`
- **Linux**：`~/.local/share/qingjian/models/hanzhang-tongbian/`

- 知意下载后在 `models/zhiyi/`（Windows 是 `models/zhiyi-onnx/`）。含章·知微已下线，`models/hanzhang-zhiwei/` 不再读取，可以删掉。
- 知意输入法的词库模型（`zh.ngram`）目前不随包，只有 Windows 版会读：放进 `%APPDATA%\Qingjian\models\zhiyi-ngram\` 后重启输入法，整句和候选排序都会参考它；它认识而词库没收的词（摸鱼、老梗这类）也能打出来。放进去后第一次启动会多等一秒左右，在同一目录生成 `zh.words.qj`；之后直接读这个文件，换了模型或升级词库时会自动重新生成，删掉也没关系。
- 旧版用过的 `model-p2c/` 与 `model/` 文件夹仍可读取；同一模型的新文件夹优先。
- 直接导出的 `model.safetensors`、`config.json`、`vocab.json` 三个文件也能识别；同一文件夹里两种都有时优先用 `.qjm`。
