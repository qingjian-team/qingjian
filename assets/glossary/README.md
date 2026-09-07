# 释义表

候选词右侧那一行「词性 + 译词」的数据，全部由 `tools/gloss-gen` 用 LLM（DeepSeek）离线批量生成，不含任何第三方词典内容，随代码许可发布。

- `data/generated/gloss-llm.jsonl`：模型原始输出，一行一个词（词性、英文译词、日文译词与假名），可续跑：
  `cargo run --release -p qingjian-gloss-gen -- generate --words assets/lexicon/dict.tsv --min-count 1 --max-chars 8`
- `glossary-en.tsv` / `glossary-ja.tsv`：输入法加载的表，`词\t词性. 译词[|假名]\t…`，由 `... export --out-dir assets/glossary` 导出。

模型原始输出的三个 JSONL（`gloss-llm.jsonl`、`gloss-en-llm.jsonl`、`pinyin-llm.jsonl`）都在 `data/generated/`，不进 git：
它们只是续跑用的中间产物，每重跑一轮就变一份，仓库只保留导出的最终表。发布时随 `.qj` 一起作为 Release 附件保存，重跑前先从那里下载。

2026-09-05 对 `assets/lexicon/dict.tsv` 全量生成：23.9 万词（含旧语料词表的 3.8 万），词库多字词 91% 有英文释义、98% 有日文释义。
- `data/generated/gloss-en-llm.jsonl` / `glossary-zh.tsv`：英→中，英文候选（中英混输、英文模式）右侧显示的中文释义。词按 wordfreq 词频 ≥ 2500 加技术词表全部，约 4.5 万词：
  `cargo run --release -p qingjian-gloss-gen -- english --include assets/lexicon/05_english/05_tech/*.tsv` 再 `... export-english`。表的键是小写，Engine 查表时把候选转小写。

生成时的提示词在 `tools/gloss-gen/src/prompt.rs`（中→英 / 日）与 `tools/gloss-gen/src/english.rs`（英→中）。
