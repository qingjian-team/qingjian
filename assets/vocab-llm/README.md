# 四家开源模型 tokenizer 词表（青简补充词库的来源）

这一份数据**不随包**：`data/generated/dicts/vocab-llm4.qj` 由 `tools/corpus/llm_tokenizer_vocab.py`
从下面四家的 tokenizer 文件里取出纯汉字 token 合并而成，许可与署名跟着这四家走。

## 来源与许可

| 来源 | 取的文件 | 许可 | 义务 |
|---|---|---|---|
| [DeepSeek-V4.1-Flash-1017](https://huggingface.co/deepseek-ai/DeepSeek-V4.1-Flash-1017) | `tokenizer.json` | MIT（[原文](licenses/deepseek-v4.1-flash-LICENSE.txt)） | 附许可与版权行 |
| [GLM-5.3](https://huggingface.co/zai-org/GLM-5.3) | `tokenizer.json` | GLM-5.3 License（自定义，[原文](licenses/glm-5.3-LICENSE.txt)） | 附许可原文与版权行；条款随下游传递 |
| [Kimi-K3](https://huggingface.co/moonshotai/Kimi-K3) | `tiktoken.model` | Kimi K3 License（自定义，[原文](licenses/kimi-k3-LICENSE.txt)） | 附许可原文与版权行；条款随下游传递 |
| [Qwen3.8-Flash-Next](https://huggingface.co/Qwen/Qwen3.8-Flash-Next) | `vocab.json` | Qwen Community License 1.0（自定义，[原文](licenses/qwen3.8-LICENSE.txt)） | 附许可原文与版权行；条款随下游传递 |
| [jieba](https://github.com/fxsjy/jieba)（另一份 `vocab-opensource.qj` 用） | `dict.txt` | MIT（[原文](licenses/jieba-LICENSE.txt)） | 附许可与版权行 |
| [mozillazg/pinyin-data](https://github.com/mozillazg/pinyin-data) | `pinyin.txt` | MIT（[原文](licenses/pinyin-data-LICENSE.txt)） | 附许可与版权行 |
| [mozillazg/phrase-pinyin-data](https://github.com/mozillazg/phrase-pinyin-data) | `pinyin.txt` | MIT（[原文](licenses/phrase-pinyin-data-LICENSE.txt)） | 附许可与版权行 |

注意：四家里只有 DeepSeek-V4.1-Flash 是标准 MIT；GLM-5.3、Kimi-K3、Qwen3.8 都是各家自己的许可
（原文里都有署名与使用限制条款），不是 Apache/MIT，SPDX 里用 `LicenseRef-` 指过去。

## 两份词表的 SPDX 写法（`dict-convert pack dict --license`）

```
四家 tokenizer 合并（vocab-llm4.qj）：
  MIT AND LicenseRef-GLM-5.3 AND LicenseRef-Kimi-K3 AND LicenseRef-Qwen-Community-1.0

jieba + mozillazg 拼音（vocab-opensource.qj）：
  MIT
```

## 怎么复现

```bash
# 下载（tokenizer 文件本来就在模型仓库里，不需要任何 API key）
#   deepseek-v4.1-flash.json / glm-5.3.json ← tokenizer.json，qwen3.8.json ← vocab.json
#   kimi-k3.model  ← Kimi-K3 的 tiktoken.model
#   pinyin-data.txt / phrase-pinyin.txt  ← mozillazg 两份数据
python tools/corpus/llm_tokenizer_vocab.py --out data/generated/vocab-llm4.tsv \
    --jsonl data/generated/vocab-llm4.jsonl
cargo run -p qingjian-gloss-gen -- export-vocab --input data/generated/vocab-llm4.jsonl \
    --existing "<装机 dict.qj>" --name vocab-llm4
cargo run -p qingjian-dict-convert -- --out-dir <临时目录> pack dict \
    --input data/generated/vocab-llm4.tsv --name "四家开源模型词表" \
    --license "MIT AND LicenseRef-GLM-5.3 AND LicenseRef-Kimi-K3 AND LicenseRef-Qwen-Community-1.0" \
    --attribution "DeepSeek-V4.1-Flash (c) 2026 DeepSeek; GLM-5.3 (c) 2026 Zhipu AI; Kimi-K3 (c) 2025 Moonshot AI; Qwen3.8-Flash-Next (c) 2026 Alibaba Group"
```

## 分发时要带什么

随包或单独分发这两份 `.qj` 时，把 `licenses/` 整个目录一起带上（七份原文），
并在产品的第三方声明里列出上表的来源、版权行与许可名。GLM / Kimi / Qwen 那三条不是标准开源许可，
原文里的使用限制/署名要求要照抄，别只写 “MIT/Apache”。
