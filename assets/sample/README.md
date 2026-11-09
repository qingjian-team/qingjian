# 样例数据

手写的小词库，只用于 CLI 演示和测试，不是产品词库（产品词库在词库仓库 qingjian-dicts）。除 `wubi86.tsv` 外不涉及第三方许可。

- `dict.tsv`：`词\t音节\t词频`
- `glossary-en.tsv` / `glossary-ja.tsv` / `glossary-vi.tsv` / `glossary-de.tsv`：`词\t[词性. ]译文[\t[词性. ]译文]`，每个文件一种学习语言（德语名词带定冠词）
- `english.tsv`：英文模式候选用的词表，`词\t编码\t词频`（与 `data/generated/english.tsv` 同格式）
- `wubi86.tsv`：五笔样例码表，`词\t编码\t词频`，取自随包 86 五笔码表（极点码表，Apache-2.0）的一级简码与测试用到的几条
