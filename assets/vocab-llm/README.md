# 四家开源模型 tokenizer 词表（青简补充词库的来源）

这一份数据**不随包**：`data/generated/dicts/vocab-llm4.qj` 由 `tools/corpus/llm_tokenizer_vocab.py`
从下面四家的 tokenizer 文件里取出纯汉字 token 合并而成，许可与署名跟着这四家走。

## 来源与许可

| 来源 | 取的文件 | 许可 | 义务 |
|---|---|---|---|
| [DeepSeek-V3](https://huggingface.co/deepseek-ai/DeepSeek-V3) | `tokenizer.json` | DeepSeek License Agreement 1.0（[原文](licenses/deepseek-v3-LICENSE-MODEL.txt)） | 随分发附许可原文、保留版权行；许可里的使用限制随下游一起传递 |
| [GLM-4.5](https://huggingface.co/zai-org/GLM-4.5) | `tokenizer.json` | Apache-2.0（[原文](licenses/glm-4.5-LICENSE.txt)） | 附许可、保留版权与 NOTICE |
| [Kimi-K2-Instruct](https://huggingface.co/moonshotai/Kimi-K2-Instruct) | `tiktoken.model` | Modified MIT（[原文](licenses/kimi-k2-LICENSE.txt)） | 附许可与版权行；月活超过 1 亿时产品界面要显示 “Kimi K2” |
| [Qwen2.5-7B-Instruct](https://huggingface.co/Qwen/Qwen2.5-7B-Instruct) | `tokenizer.json` | Apache-2.0（[原文](licenses/qwen2.5-LICENSE.txt)） | 附许可、保留版权与 NOTICE |
| [jieba](https://github.com/fxsjy/jieba)（另一份 `vocab-opensource.qj` 用） | `dict.txt` | MIT（[原文](licenses/jieba-LICENSE.txt)） | 附许可与版权行 |
| [mozillazg/pinyin-data](https://github.com/mozillazg/pinyin-data) | `pinyin.txt` | MIT（[原文](licenses/pinyin-data-LICENSE.txt)） | 附许可与版权行 |
| [mozillazg/phrase-pinyin-data](https://github.com/mozillazg/phrase-pinyin-data) | `pinyin.txt` | MIT（[原文](licenses/phrase-pinyin-data-LICENSE.txt)） | 附许可与版权行 |

注意：GLM-4.5 在 HuggingFace 的卡片上标的是 `mit`，仓库里的 `LICENSE` 文件是 Apache-2.0。
这里按**文件**算（Apache-2.0），SPDX 表达式也就写 Apache-2.0。

## 两份词表的 SPDX 写法（`dict-convert pack dict --license`）

```
四家 tokenizer 合并（vocab-llm4.qj）：
  Apache-2.0 AND MIT AND LicenseRef-DeepSeek-License-1.0 AND LicenseRef-Kimi-K2-Modified-MIT

jieba + mozillazg 拼音（vocab-opensource.qj）：
  MIT
```

## 怎么复现

```bash
# 下载（tokenizer 文件本来就在模型仓库里，不需要任何 API key）
#   deepseek.json / glm.json / qwen.json  ← 各家的 tokenizer.json
#   kimi-tiktoken.model                  ← Kimi-K2 的 tiktoken.model
#   pinyin-data.txt / phrase-pinyin.txt  ← mozillazg 两份数据
python tools/corpus/llm_tokenizer_vocab.py --out data/generated/vocab-llm4.tsv \
    --jsonl data/generated/vocab-llm4.jsonl
cargo run -p qingjian-gloss-gen -- export-vocab --input data/generated/vocab-llm4.jsonl \
    --existing "<装机 dict.qj>" --name vocab-llm4
cargo run -p qingjian-dict-convert -- --out-dir <临时目录> pack dict \
    --input data/generated/vocab-llm4.tsv --name "四家开源模型词表" \
    --license "Apache-2.0 AND MIT AND LicenseRef-DeepSeek-License-1.0 AND LicenseRef-Kimi-K2-Modified-MIT" \
    --attribution "DeepSeek-V3 (c) 2023 DeepSeek; GLM-4.5 (c) 2025 Zhipu AI; Kimi-K2 (c) 2025 Moonshot AI; Qwen2.5 (c) 2024 Alibaba Group"
```

## 分发时要带什么

随包或单独分发这两份 `.qj` 时，把 `licenses/` 整个目录一起带上（七份原文），
并在产品的第三方声明里列出上表的来源、版权行与许可名。DeepSeek 与 Kimi 那两条不是标准开源许可，
原文里的使用限制/署名要求要照抄，别只写 “MIT/Apache”。
