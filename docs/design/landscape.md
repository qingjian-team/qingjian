# 同类项目与数据源

## 水杉输入法（MetasequoiaIME）

- https://github.com/metasequoiaime/MetasequoiaImeTsf
- Windows 专用，纯 TSF，自研引擎（非 Rime），GPL-3.0。截至 2026-09 约 950 star，近期新增 5000+ 用户。
- 已经实现了「候选旁显示译文」：竖排候选窗中为候选项显示中文与**所选语种**的互译，每项最多两条简短释义；
  本地词典优先，未命中可走腾讯云 TMT 或自建 DeepLX 兼容服务。
- 另有日文罗马字输入模式、中英混输。
- issue 里翻译功能相关的反馈占比不低（错译、想直接上屏译文、日语学习模式加罗马音、短语翻译不准），
  说明用户在实际使用这个功能。

对青简的意义：

- 「打字时看到译文」这个交互形态已经被水杉验证，不需要再做原型验证。
- 青简相对水杉的差异只剩三条：非 Windows 平台（水杉没有 macOS / Linux）、Rust 平台无关 Core、
  更克制的单条 annotation（水杉最多两条，可走云端）。
- 青简的输入法本体需要先达到「不比水杉差」这条线，翻译功能才有意义。
- 水杉的 TSF 代码可以作为 Phase 5 的参考，但它是 GPL-3.0，除非青简也选 GPL，否则不能搬。
- 水杉的进程结构（TSF DLL + 独立 Server 进程 + UI）是 Windows 平台层应采用的结构。

## Rime

- https://rime.im/
- librime（C++，约 5 万行）+ 各平台前端：Squirrel（macOS）、Weasel（Windows）、ibus-rime / fcitx-rime（Linux）。
- 青简的「Core + 薄壳」架构与 Rime 相同，Rime 证明了这个结构能跑十几年。
- Rime 候选自带 comment 字段，Squirrel 显示在候选旁，配合 librime-lua 也能做出类似的译文标注。

## 数据源

| 用途 | 来源 | 许可 | 备注 |
|---|---|---|---|
| 拼音词库（产品） | 自建，源数据在 `assets/lexicon/`：《通用规范汉字表》8105 字（政府公布的规范字表，iDvel 转录并与 shengdoushi 分级表核对）、《现代汉语常用词表（草案）》56k 词（教育部 2008，liuxilu 校对版，自带拼音）、THUOCL 11 类领域词 15.7 万（2026-09-06 起按语料次数拆开：常见的留基础词库，其余各成一本随包 `.qj`；清华 NLP，MIT） | 字表与常用词表是官方文件的转录，转录仓库未附许可证；THUOCL MIT；读音 Unihan（Unicode License v3） | `dict-convert lexicon` 生成 `dict.tsv`（20.5 万条，随仓库放 `assets/lexicon/dict.tsv`）：字与领域词读音来自 Unihan，5.6 万含多音字的领域词和 1.2 万含多音字的常用词由 LLM 标注再逐字对照 Unihan 校验（常用词表自带拼音对多音字错了 288 处：部长 chang、西藏 cang、成都 dou……，以标注为主读音、原读音降权保留），词频来自下面两份语料的统计（两遍：先按底值分词统计，再用真实词频重统计）。2026-09-05 起取代雾凇拼音（GPL-3.0 only，与本项目不兼容，已从数据目录与工具里移除） |
| 英文词表（产品） | 同一数据包 `05_english`：ESDB（en-wl/wordlist，MIT-like）、CSpell software-terms（MIT）、typos 误拼对照（MIT） | 均宽松，许可原文随包 | `dict-convert english` 取纯字母词 9.5 万，词频 wordfreq；误拼对照表以后可喂英文纠正。取代雾凇 en_dicts |
| 语言模型语料 | 中文维基（HF `wikimedia/wikipedia` 20231101.zh） | CC BY-SA 4.0 | 百科体，第一人称、口语词几乎没有（去 / 累 这类词计数极低），单独用它整句会偏向地名术语；由它统计的 bigram 表按 CC BY-SA 对待 |
| 语言模型语料 | LCCC（清华 `thu-coai/lccc`，微博对话） | MIT | 口语对话，补维基缺的日常用语；两份语料合并统计。`tools/corpus/parquet_to_text.py` 转文本，`dict-convert bigram` 统计 |
| 语言模型 | Rime 八股文 / octagram | essay 只是词频表；octagram 是 LGPL | 未采用：essay 没有二元信息，octagram 文件格式是 Rime 私有的 |
| 中→英 / 中→日 | `tools/gloss-gen` 用 LLM（DeepSeek）离线批量生成，数据在 `assets/glossary/` | 模型输出，许可干净 | 词性 + 译词 + 日文假名；2026-09-05 对整个自建词库跑了全量：23.9 万词，词库多字词 91% 有英文释义、98% 有日文释义，单字 92%；17.6 万词约 100 分钟、几十元。有道等网页词典接口不用：逆向的签名接口不稳，攒下来的结果再分发是侵权 |
| 中→英（备用） | CC-CEDICT | CC BY-SA 4.0 | 需署名；share-alike 只约束数据本身及其加工版，不传染代码。**没有词性**、释义偏长，已被 LLM 表取代，`dict-convert cedict` 仍能生成 |
| 日文读音 / 词性（候选） | JMdict（EDRDG） | CC BY-SA 4.0 | 以后校对 LLM 给的假名与日文词性 |
| 词汇等级（统计） | 已用：CEFR-J Wordlist 1.5（A1–B2，免费须署名）+ Octanove C1/C2（CC BY-SA 4.0）；JLPT N5–N1（Tanos CC BY，经 elzup MIT 整理）。四六级 / 商务英语词表在 GitHub 上只有大纲词汇的转录、许可不明，没用 | 已核 | 「统计」页按级数见过 / 看熟 / 上屏过的译词（`assets/levels/`）；也可做英→中 / 日→中方向的种子；不进候选窗口 |
| emoji | Unicode CLDR 中文与英文 annotations（`cldr-json` 的 `annotations/{zh,en}`、`annotationsDerived/{zh,en}`） | Unicode License v3（宽松，需保留版权声明） | `dict-convert emoji --language zh\|en` → `assets/emoji/emoji-{zh,en}.tsv`，随仓库与发布包提供；许可文本在 `assets/emoji/LICENSE-unicode.txt`。日文表要等有日语输入模式 |
| 英文词频 | wordfreq（Python 包） | 代码 MIT，数据 CC-BY-SA 等 | 只取每个词的 Zipf 频率数字给英文补全排序，`tools/corpus/english_frequency.py` 生成 `english-frequency.tsv`（gitignore） |

## 许可策略

- 代码：GPL-3.0-or-later（2026-09-07 定，此前测试阶段是「保留所有权利」）。选强 copyleft 是为了堵闭源抄走，与鼠须管 / 水杉一致；名字与 logo 不授权。
  `LICENSE`、「关于」页（`about.rs` 的 `LICENSE_NOTE`）、README 三处保持一致；pkg 里的 `license.txt` 从 `LICENSE` 拷。
- 产品数据的源文件进 `assets/`（`assets/lexicon/` 词库源、`assets/emoji/`），带各自的许可证与署名文件；
  语料、Unihan 这类体积大或可重新下载的中间输入放仓库根目录 `data/`（gitignore），生成物在 `data/generated/`。
  许可与本项目不兼容的数据（雾凇拼音）已不再使用，连开发测试也不用，免得产品数据被它污染。
- 数据文件不并入代码许可：每个数据源在 `assets/` 下带自己的 LICENSE 和署名文件，README 里列出。
  CC BY 只需署名；CC BY-SA 的 share-alike 只约束数据本身及其修改版，不传染到代码，
  但对 CC-CEDICT 做的任何加工（裁剪、合并 gloss）都要以 CC BY-SA 发布。
- 每新增一个数据源或依赖，同步更新上表和 README 的 License 一节。
- GPL 项目（水杉、部分 Rime 前端）的代码只能读不能搬。
