# 青简词库源数据

这是青简自己的词库来源，从「输入法字词库_分类整理版」数据包拷入（2026-09-05），只去掉了与 TSV 内容相同的 TXT 副本；
目录与文件名改成了 ASCII（GitHub 链接、各平台 shell 都省事），对照：`00_说明与校验 → 00_meta`、`01_标准字库 → 01_characters`、
`02_通用词库 → 02_common`、`03_领域词库 → 03_domains`、`04_网络用语_待采集 → 04_internet_slang`、`05_英文词库 → 05_english`，
文件名同理（`现代汉语常用词 → modern_chinese_common_words`、领域按英文名）。`00_meta/SHA256SUMS.txt` 里记的是原始文件名与校验值。
数据包自身的说明见 [README.md](README.md)，英文部分见 [05_english/README.md](05_english/README.md)，
各来源许可证原文在 `00_meta/` 与 `05_english/sources/`。

## 怎么变成输入法用的词库

```bash
# 1. 第一遍：写初版 dict.tsv，并列出要交给 LLM 定读音的词：没有拼音的领域词里含多音字的，以及常用词表里含多音字的
#    （常用词表自带的拼音对多音字有错，如 重庆 zhong4'qing4，所以也要标；标注与原表不同时标注为主读音，原表读音降权保留）
cargo run --release -p qingjian-dict-convert -- lexicon --emit-ambiguous data/generated/lexicon-ambiguous.txt
# 2. 多音字词交给 LLM 标读音（可中断续跑；结果 data/generated/pinyin-llm.jsonl，不进 git，发布时作为 Release 附件保存）
cargo run --release -p qingjian-gloss-gen -- pinyin
# 3. 用初版词库分词、统计语料词频（语料在 data/corpus/，见 docs/design/landscape.md）
cargo run --release -p qingjian-dict-convert -- bigram data/corpus/*.txt
# 4. 第二遍：带标注与词频写最终 dict.tsv；再统计一次语料让分词用上真实词频
#    领域词同时拆出：语料里 ≥ 50 次（--domain-keep-min）的留在 dict.tsv，其余按来源文件各写一本 dicts/<领域>.tsv + dicts/<领域>.qj（带 META）；
#    bigram / mine 分词时会自动把 dicts/*.tsv 一起当词表，所以拆分不影响语言模型
cargo run --release -p qingjian-dict-convert -- lexicon --pinyin data/generated/pinyin-llm.jsonl --frequency data/generated/lm-unigram.tsv
cargo run --release -p qingjian-dict-convert -- bigram data/corpus/*.txt
# 4b. 语料挖词库没收的高频词（分词落成连续单字的段），mine 自带虚词规则 + 相邻字对 PMI≥3 过滤（--min-pmi 调，
#     --candidates 可跳过扫语料只重过滤）：写 oov-candidates.tsv（原始）、oov-filtered.tsv（过滤后，拷成 assets/lexicon/mined_words.tsv）、
#     oov-words.txt（交 gloss-gen pinyin 标音）；lexicon 加 --extra-words 并入，然后重跑一次 bigram。
#     注意 PMI 用当前的一元表：并入挖出的词并重跑 bigram 之后单字次数会变，再挖一遍结果不同是正常的
cargo run --release -p qingjian-dict-convert -- mine data/corpus/*.txt
cp data/generated/oov-filtered.tsv assets/lexicon/mined_words.tsv
cargo run --release -p qingjian-dict-convert -- lexicon --pinyin data/generated/pinyin-llm.jsonl --frequency data/generated/lm-unigram.tsv --extra-words assets/lexicon/mined_words.tsv
# 5. 英文词表
cargo run --release -p qingjian-dict-convert -- english assets/lexicon/05_english/00_all_words.tsv
uv run tools/corpus/english_frequency.py data/generated/english.tsv -o data/generated/english-frequency.tsv
cargo run --release -p qingjian-dict-convert -- english assets/lexicon/05_english/00_all_words.tsv --frequency data/generated/english-frequency.tsv
# 6. 打包（bundle.sh 会自动做；领域词库的 .qj 第 4 步已经写好，bundle.sh 直接拷进 Resources/dicts/）
cargo run --release -p qingjian-dict-convert -- pack dict --name 青简基础词库 --license "MIT AND Unicode-3.0"
```

读音来自 Unihan（`data/unihan/Unihan_Readings.txt`，从 https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip 解出，Unicode License v3）；
多音字词的读音由 LLM 标注后逐字对照 Unihan 校验。最终的基础词库 `dict.tsv` 与拆出的领域词库 `dicts/*.tsv` 也随仓库放在这个目录，没有语料和 API 也能直接打包。
