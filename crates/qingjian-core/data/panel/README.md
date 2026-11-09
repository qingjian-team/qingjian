# 符号面板数据

`tools/corpus/symbol_panel.py` 生成，编进 `qingjian-core`。

| 文件 | 格式 | 内容 |
|---|---|---|
| `catalog.tsv` | `分组\t分类\t条目\t名字` | 按行序排出面板的分类与条目；名字是 CLDR 中文名（颜文字取它的中文叫法），可能为空 |
| `keywords.tsv` | `词\t条目` | 搜索用：CLDR 中文关键词与名字、符号库的检索词（两个字以上的）、常用表情的中文叫法 |

来源：符号与分类是维护者整理的符号库（Unicode 17.0 字符）与日文假名表，「网络」一类（`.com`、`http://` 这类整段）写死在生成脚本里；emoji 与分组取自 Unicode emoji-test.txt
（只收完全限定、不带肤色、Emoji 15.1 以内的，再新的旧系统显示不出来）；名字与关键词取自 Unicode CLDR 中文 annotations
（Unicode License v3，见 `assets/emoji/LICENSE-unicode.txt`）。
