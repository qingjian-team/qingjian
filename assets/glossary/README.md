# 释义表

候选词右侧那一行「词性 + 译词」的数据，一张表一门学习语言。

格式：`词\t[词性. ]译词[|读音]\t…`，UTF-8 **无 BOM**、LF 换行、按码点排序。
解析在 `crates/qingjian-translate/src/glossary/mod.rs`，规则是严格的：任一行缺词或缺释义，
整张表加载失败（`.qj` 打包走同一套解析）。

本目录各表随代码以 **GPL-3.0-or-later** 发布，与仓库一致。各表来源不同，见下。

## 英语 / 日语（`glossary-en.tsv` / `glossary-ja.tsv`）

由 `tools/gloss-gen` 用 LLM（DeepSeek）离线批量生成，不含任何第三方词典内容。

- `data/generated/gloss-llm.jsonl`：模型原始输出，一行一个词（词性、英文译词、日文译词与假名），可续跑：
  `cargo run --release -p qingjian-gloss-gen -- generate --words assets/lexicon/dict.tsv --min-count 1 --max-chars 8`
- `glossary-en.tsv` / `glossary-ja.tsv`：输入法加载的表，由 `... export --out-dir assets/glossary` 导出。

生成工具的英文释义池最多保留六条，常用表达在前，同一义项的书面或进阶同义词在后；日语仍最多两条。

`learning-en.tsv` 是人工维护的常用中文词学习译词补充，同为 GPL-3.0-or-later。
`export` 在输出目录存在此文件时，把补充按原顺序追加到对应英语条目后并去重；原有常用译词保持在前，重导出不会丢失这些补充。
补充包含较书面或不常用的同义表达；难度使用既有 CEFR 等级表，未分级的词只在「全部」或无目标等级的兜底池中出现。
释义池不再只保留前两条，TSV → `.qj` 打包保留全部；候选窗经难度筛选、随机选择后仍最多显示两条。

2026-09-05 对 `assets/lexicon/dict.tsv` 全量生成：23.9 万词（含旧语料词表的 3.8 万），
词库多字词 91% 有英文释义、98% 有日文释义。

## 西班牙语（`glossary-es.tsv`）

由 `tools/corpus/glossary_es.py` 用 **Azure Translator** 机器翻译生成，**不是 LLM**，
也不经过 `gloss-gen` 的 `export`（那只写 en / ja 两张表），是这个脚本的直接产物。

- 词表、词性、英文释义都取自 `glossary-en.tsv`；译文来自 Azure Translator F0 免费层
  （每月 200 万字符，跑完全量用掉约 127 万）。
- 可复现，有 Azure 免费 key 就能重跑：
  `AZURE_TRANSLATOR_KEY=… AZURE_TRANSLATOR_REGION=… uv run tools/corpus/glossary_es.py`
- 覆盖：`glossary-en.tsv` 的 232,213 个词里 193,292 个有西语释义（83.2%）。
- 翻译策略（按词性分源语言）与三组对照实验记在脚本的模块 docstring 里。

**已知问题**（2026-09-14 生成，机器翻译，未经人工校对）：

- 名词约 13% 还是复数（`朋友 -> amigos`，不是 `amigo`）
- 个别错译：`椅子 -> presidente`、`工作 -> obra`、`必须 -> debe`
- 生僻字与专名的准确率低
- 动词走英文源（`to {英文释义} (verb)`）后约 86% 是原形，其余是别的动词形式
- 有一部分条目的译词还是英文，没有翻成西语：
  - 整条是英文的：「会 + 动词」这一类几乎全是（`会买 -> will buy`、`会去 -> will go`、
    `会飞 -> can fly`），另有 `你需要 -> you need`、`下了 -> got off`、`不得 -> must not`、
    `中耳炎 -> otitis media`。这类全小写的英文短语共 177 条
  - 单个词没翻的：`三明治 -> sandwich`、`五金 -> hardware`
  - 英西混排的：`也会 -> will también`、`会令 -> will hacer`、`不送 -> no Ver Off`
  - 与 `glossary-en.tsv` 的英文释义逐字相同的条目里，绝大多数是同形词或专名
    （`abdomen`、`vector`、`Ding Wei`），属正常
- 7 条没有词性（源表 `glossary-en.tsv` 里就没有），如 `刷拉 -> swish`、`嗡嗡 -> buzz`、
  `御 -> carruaje imperial`；其中 `第 -> No` 是错译（源表的英文释义是 `-th`）
- 短语中间的词首字母有时会被无端大写：`开发区 -> zona de Desarrollo`、`一等奖 -> primer Premio`

欢迎报错译：有反馈之后可以再用 LLM 在现有表上精修一轮。

## 英→中（`glossary-zh.tsv`）

英文候选（中英混输、英文模式）右侧显示的中文释义。词按 wordfreq 词频 ≥ 2500 加技术词表全部，
约 4.5 万词：

```
cargo run --release -p qingjian-gloss-gen -- english --include assets/lexicon/05_english/05_tech/*.tsv
cargo run --release -p qingjian-gloss-gen -- export-english
```

`data/generated/gloss-en-llm.jsonl` 是它的原始输出。表的键是小写，Engine 查表时把候选转小写。

## 中间产物

模型原始输出的三个 JSONL（`gloss-llm.jsonl`、`gloss-en-llm.jsonl`、`pinyin-llm.jsonl`）都在
`data/generated/`，不进 git：它们只是续跑用的中间产物，每重跑一轮就变一份，仓库只保留导出的最终表。
发布时随 `.qj` 一起作为 Release 附件保存，重跑前先从那里下载。

生成时的提示词在 `tools/gloss-gen/src/prompt.rs`（中→英 / 日）与 `tools/gloss-gen/src/english.rs`（英→中）。
