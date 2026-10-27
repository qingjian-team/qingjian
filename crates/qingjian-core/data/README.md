# Core 内置的 emoji 补充、颜文字、符号表与单字声调表

`include_str!` 编进 `qingjian-core`，不随包另带文件。格式一行一条 `词\t条目`，第二列整列是一条（颜文字里有空格）；同一个词的多条按行序排，候选按这个顺序出。

| 文件 | 内容 | 来源 |
|---|---|---|
| `emoji.tsv` | CLDR 没有的口语叫法（笑死 → 🤣、无语 → 🙃）与少量英文别名 | 维护者整理的常用 emoji 表；emoji 本身是 Unicode 字符 |
| `kaomoji.tsv` | 词 → 颜文字（哭 → (T_T)） | 维护者整理的颜文字选编 |
| `symbols.tsv` | 词 → 符号（约等于 → ≈、alpha → α） | 符号是 Unicode 17.0 字符；中文词为本项目整理，英文词取自希腊字母的 Unicode 名 |

`tones.tsv` 是注音按声调排序用的单字声调表（`字\t带调音节…`，轻声记 5），由 `qingjian-dict-convert --out-dir crates/qingjian-core/data tones` 从 `assets/lexicon/02_common` 的通用词表生成，不手改。

`words.tsv` 是上面三张表里词库没有的词（尖括号、笑死、阿尔法），格式同词库 `词\t音节\t词频`，词频统一 300：不收的话整句会拼成「见括号」，配不上符号。表里加了新词，查一下词库有没有，没有就在这里补读音。

`chaizi.tsv` 是问字模式按部件读音找字的拆字表（`字\t读音…\t部件拼音串…`，按字频排），由 `qingjian-dict-convert --out-dir crates/qingjian-core/data chaizi` 从 `assets/chaizi`（開放詞典網拆字字典，CC BY 3.0）与 `assets/lexicon/dict.tsv` 生成，不手改。

符号表的中文词只收两个字以上的：单字（上、左、角）一打就跟符号，太吵。CLDR 主表在 `assets/emoji/`。
