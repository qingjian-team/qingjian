# 许可清单

青简装到用户机器上的每样东西各是什么许可、原文在哪、还有哪些没理清。给两类人看：
维护者（新增数据源、发版时对照），以及要把青简收进发行版软件仓库的打包者。

- 仓库里每个文件的许可：根目录的 `REUSE.toml`（机器可读，`reuse lint` 校验），许可原文在 `LICENSES/`。
- 不在仓库里、发版时才下载的产品数据包与模型：只记在本页「产品数据包」「模型」两节。
- 选数据源时的取舍与理由：[design/landscape.md](../design/landscape.md) 的「许可策略」「随包数据清单」。

**草稿**（2026-10-16 起草）：「待定事项」里的几项没有定之前，本页与 `REUSE.toml` 里对应的条目都只是现状记录，不代表已经取得授权。

## 总览

| 部分 | SPDX 许可表达式 | 状态 |
|---|---|---|
| 代码（全部 crate 与各平台壳） | `GPL-3.0-or-later` | 清楚 |
| `bamboo-slip`、`qingjian-render` 的源代码 | `GPL-3.0-or-later OR LicenseRef-Qingjian-Commercial` | 清楚；打包按 GPL 那一支即可 |
| 第三方 Rust 依赖（Linux 端约 450 个） | MIT / Apache-2.0 / BSD / Zlib / ISC / Unicode-3.0 等宽松许可 | 清楚，都与 GPL-3.0 兼容；随包的许可原文还没生成（待定事项 6） |
| 主题里的角色图片 | `LicenseRef-Character`、`LicenseRef-PCL` | **未授权 / 仅限非商业**（待定事项 1） |
| 名称与图标 | `LicenseRef-Qingjian-Logo` | **未授权**（待定事项 2） |
| 含章模型权重 | 无 | **未定**（待定事项 3） |
| 词库里的两份字词表 | `LicenseRef-NoLicense` | **上游没附许可**（待定事项 4） |
| CEFR-J 词汇等级 | `LicenseRef-CEFR-J` | **条款不完整**（待定事项 5） |
| 其余随包数据 | MIT、Apache-2.0、Unicode-3.0、CC-BY-4.0、CC-BY-SA-4.0、OGDL-Taiwan-1.0、OFL-1.1、`LicenseRef-ESDB` | 清楚，按各自要求署名 |

## 可执行文件

| 文件 | 内容 | 许可 |
|---|---|---|
| `qingjian-linux-server`、`qingjian-linux-ibus`、Windows Server / TSF DLL、macOS 输入法 | 青简代码 + Rust 依赖 | `GPL-3.0-or-later`，依赖见下 |
| 同上，编进程序的数据 | `crates/qingjian-core/data/`（emoji 口语叫法、颜文字、符号，维护者整理） | `GPL-3.0-or-later` |
| 同上，编进程序的数据 | 简繁转换字典（OpenCC，经 `ferrous-opencc` 编译期打进产物） | `Apache-2.0` |
| `qingjian-settings`、macOS 输入法、Windows Server（链接 `qingjian-render`） | 内置主题，图片用 `include_bytes!` 编进程序 | `GPL-3.0-or-later`；**其中 miku / bocchi / dafeiyu 三个主题的人物图另见待定事项 1** |

第三方 Rust 依赖的许可用 `cargo metadata` 统计（2026-10-16，Linux 端 Server、IBus 前端与设置程序实际链接的部分）：
446 个 crate，没有 GPL-only、非自由或「保留所有权利」的；`Apache-2.0 OR GPL-2.0-only` 这类可选许可的按 Apache-2.0 用。

## 随包资源（`share/qingjian/resources/assets/`）

| 目录 / 文件 | 来源 | 许可 |
|---|---|---|
| `sample/` | 维护者手写的演示词库 | `GPL-3.0-or-later` |
| `glossary/glossary-{en,ja,vi,de}.tsv` | `tools/gloss-gen` 用 LLM 生成（英 / 日 / 德：DeepSeek；越：gpt-5.5），不含第三方词典内容；德语冠词只拿 Wiktionary 快照（CC BY-SA 4.0）做校验、不取内容 | `GPL-3.0-or-later` |
| `glossary/glossary-es.tsv` | Azure Translator 由英文释义机器翻译 | `GPL-3.0-or-later` |
| `glossary/glossary-zh.tsv` | LLM 生成；选词按 wordfreq 词频与 CSpell 技术词表，只用到词形 | `GPL-3.0-or-later` |
| `levels/levels-en.tsv` | CEFR-J Wordlist 1.5（A1–B2）+ Octanove Vocabulary Profile C1/C2 | `LicenseRef-CEFR-J AND CC-BY-SA-4.0` |
| `levels/levels-ja.tsv` | Tanos JLPT 词表，经 elzup/jlpt-word-list 整理 | `CC-BY-4.0 AND MIT`（Tanos 标的是 CC BY，版本号待核） |
| `emoji/` | Unicode CLDR annotations | `Unicode-3.0`，原文 `assets/emoji/LICENSE-unicode.txt` |

## 产品数据包（`data-vN` Release 的 `qingjian-data.tar.gz`）

不在仓库里，`tools/release/data-bundle.sh` 打包、`data-fetch.sh` 按 `tools/release/data.lock` 下载校验。
**压缩包里目前没有许可说明**，下表就是要放进去的内容（待定事项 7）。

| 文件 | 由什么生成 | 许可 |
|---|---|---|
| `data/generated/dict.qj`、`dicts/*.qj` | `assets/lexicon/` 的字表与常用词表（无许可）、THUOCL 领域词（MIT）、jieba 词表（MIT）、读音取自 Unihan（Unicode-3.0）与 CNS 全字库（OGDL-Taiwan-1.0）、多音字由 LLM 标注、词频统计自中文维基（CC BY-SA 4.0）与 LCCC（MIT） | `GPL-3.0-or-later AND LicenseRef-NoLicense AND MIT AND Unicode-3.0 AND OGDL-Taiwan-1.0 AND CC-BY-SA-4.0` |
| `data/generated/lm.qj` | 中文维基百科（CC BY-SA 4.0）与 LCCC（MIT）的二元统计 | `CC-BY-SA-4.0 AND MIT` |
| `data/generated/glossary-*.qj` | 同上「随包资源」的 `glossary/*.tsv` | `GPL-3.0-or-later` |
| `data/generated/english.tsv` | ESDB 与 CSpell 词表，词频取 wordfreq 的 Zipf 值 | `LicenseRef-ESDB AND MIT AND CC-BY-SA-4.0` |
| `data/generated/english-frequency.tsv` | wordfreq 的 Zipf 值 | `CC-BY-SA-4.0`（wordfreq 数据许可的版本待核） |
| `data/generated/codes/stroke.qj` | CNS11643 全字库的笔顺资料（数位发展部），按大陆笔顺规范改写 | `OFL-1.1 AND GPL-3.0-or-later`，原文与署名在 `assets/stroke/LICENSE-CNS11643.txt` |
| `data/models/hanzhang-*/*.qjm` | 含章·通变、含章·知微 | **未定**（待定事项 3） |

CC BY-SA 4.0 的数据与 GPL-3.0 合在一个文件里：Creative Commons 已认定 CC BY-SA 4.0 可以单向兼容到 GPLv3，
派生出的统计表按两者一起列出最稳妥。

## 其他随包文件

| 文件 | 许可 |
|---|---|
| `share/licenses/qingjian/LICENSE` | GPL-3.0 原文 |
| `share/licenses/qingjian/LICENSE-CNS11643.txt` | 笔画数据的署名与 OFL-1.1 原文 |
| hicolor 图标、macOS / Windows 应用图标、`apps/settings/assets/logo.svg` | `LicenseRef-Qingjian-Logo`（待定事项 2） |
| `qingjian-settings.desktop`、Fcitx5 / IBus 的注册文件、systemd 单元 | `GPL-3.0-or-later` |

## 待定事项

前五项要维护者拿主意或联系第三方；后两项是整理工作。打包者在这些事项解决之前，按「给打包者」一节的保守写法处理。

1. **角色主题。** miku 的人物图按 Piapro Character License 只许非商业用途；bocchi、dafeiyu 的人物图没有任何授权。
   它们编进了设置程序和 macOS、Windows 的输入法，所以整个二进制包都带着这部分。多数发行版仓库要求可自由再分发（含商用），
   「仅限非商业」和「未授权」都过不了。可选的做法：加一个缺省打开的 cargo feature（例如 `character-themes`），发行版构建时关掉；
   或者把这三个主题挪到可选的单独下载。`LICENSES/LicenseRef-PCL.txt` 里还没拷 PCL 原文。
2. **名称与图标。** README 写着「项目名称与 logo 不包含在代码授权中」，但安装包里带着图标。需要一份政策：图标按什么许可提供，
   未修改的版本能不能用「青简」的名字分发。没有这份政策，打包者只能去掉图标或改名发行。
3. **含章模型权重。** [neural-rescoring.md](neural-rescoring.md) 里写的是「权重许可另定」。没有许可就不能再分发；
   定的时候连同训练数据的来源一起写，训练语料里有 CC BY-SA 的，许可最好跟着它。
4. **两份没附许可的字词表。** 通用规范汉字表（iDvel 转录）与现代汉语常用词表（liuxilu 校对版）的转录仓库都没有许可证，
   词库 `dict.qj` 由它们生成。请上游补许可，或写明法律依据（见 `LICENSES/LicenseRef-NoLicense.txt`），或换来源。
5. **CEFR-J。** 发布方只说「研究与商业用途免费，须署名」，没说能否修改与再分发。要向发布方书面确认，
   或把英语等级表做成可以不装的部分。`LICENSES/LicenseRef-CEFR-J.txt` 里还没拷发布页原文。
6. **第三方 Rust 依赖的许可原文。** MIT 与 Apache-2.0 都要求二进制分发时附许可原文。用 `cargo about` 之类的工具生成一份
   第三方许可清单，随包安装、在设置程序的「关于」页能打开。
7. **数据包里的许可说明。** `data-bundle.sh` 打包时把上面「产品数据包」那张表（连同用到的许可原文）一起放进压缩包，
   单独下载数据包的人也能看到。

另外两项不算许可，但发行版通常也会要求：

- 打包版本关掉「检查更新」（`qingjian-update`），更新由软件仓库负责。
- 云联想默认关闭、要用户自己填接口与密钥，这一点已经满足。

## 给打包者

以下写法是按各家仓库的通行做法起的草稿，**没有对照各家最新的打包文档核实**，提交前以各家文档为准。
「待定事项」1–5 解决之前，最保守的做法是：构建时去掉角色主题，不带含章模型与 CEFR-J 等级表，图标按待定事项 2 的结论处理。

- **FreeBSD Ports**：`LICENSE= GPLv3+ MIT APACHE20 ...`（各许可在框架里登记的名字以 `Mk/bsd.licenses.db.mk` 为准）、`LICENSE_COMB= multi`；
  ESDB、OGDL-Taiwan-1.0 这类框架里没有登记的许可，用 `LICENSE_NAME_<名字>` 加 `LICENSE_FILE_<名字>` 写，
  再用 `LICENSE_PERMS_<名字>` 写明允许的范围（允许分发与出二进制包时写 `dist-mirror dist-sell pkg-mirror pkg-sell auto-accept`）。
- **OpenBSD Ports**：`PERMIT_PACKAGE= Yes`，Makefile 注释里列出许可；有「仅限非商业」的内容就不能是 Yes。
- **NetBSD pkgsrc**：`LICENSE= gnu-gpl-v3 AND mit AND apache-2.0 AND ...`；pkgsrc 没有收录的许可要在 `licenses/` 下补文件，
  不在缺省接受名单里的许可会让批量构建跳过这个包。

## 维护

- 新增数据源或依赖：改 `REUSE.toml`、补 `LICENSES/` 里缺的原文、改本页对应的表，再同步
  [landscape.md](../design/landscape.md) 的「随包数据清单」与各平台「关于」页。
- 提交前跑一次 `reuse lint`（`pipx run --spec reuse reuse lint`），要求通过。
