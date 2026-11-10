# Core 内置的 emoji 补充、颜文字、符号表、单字声调表与五笔字根图

`include_str!` 编进 `qingjian-core`，不随包另带文件。格式一行一条 `词\t条目`，第二列整列是一条（颜文字里有空格）；同一个词的多条按行序排，候选按这个顺序出。

| 文件 | 内容 | 来源 |
|---|---|---|
| `emoji.tsv` | CLDR 没有的口语叫法（笑死 → 🤣、无语 → 🙃）与少量英文别名 | 维护者整理的常用 emoji 表；emoji 本身是 Unicode 字符 |
| `kaomoji.tsv` | 词 → 颜文字（哭 → (T_T)） | 维护者整理的颜文字选编 |
| `symbols.tsv` | 词 → 符号（约等于 → ≈、alpha → α） | 符号是 Unicode 17.0 字符；中文词为本项目整理，英文词取自希腊字母的 Unicode 名 |

`tones.tsv` 是注音按声调排序用的单字声调表（`字\t带调音节…`，轻声记 5），由 `qingjian-dict-convert --out-dir crates/qingjian-core/data tones` 从 `../qingjian-dicts/official/upstream/02_common` 的通用词表生成，不手改。

`words.tsv` 是上面三张表里词库没有的词（尖括号、笑死、阿尔法），格式同词库 `词\t音节\t词频`，词频统一 300：不收的话整句会拼成「见括号」，配不上符号。表里加了新词，查一下词库有没有，没有就在这里补读音。

`chaizi.tsv` 是问字模式按部件读音找字的拆字表（`字\t读音…\t部件拼音串…`，按字频排），由 `qingjian-dict-convert --out-dir crates/qingjian-core/data chaizi` 从 `../qingjian-dicts/official/upstream/chaizi`（開放詞典網拆字字典，CC BY 3.0）与 `../qingjian-dicts/official/lexicon/zh/base/words.tsv` 生成，不手改。

`parts.tsv` 是问字模式的部件别名表，手工维护（与仓库相同，GPL-3.0-or-later）。三列 `部件<TAB>别名读音（空格分隔）<TAB>笔顺（空格分隔的 h/s/p/n/z）`，只收三类：

1. **俗名与字表读音不同的**——`囗` 的字表读音是 `wei`，可用户按 `kou` 打（`ukoumu` → 困）；
2. **通用规范字表与繁体拆字表都没覆盖的字**（`馬` `長` `糹` 这些）——CNS 有笔顺，
   生成 `stroke.tsv` 时把本文件当 `stroke --filter` 的一张表传进去就能补上（命令见 `../qingjian-dicts/official/upstream/chaizi/README.md`），笔顺列留空；
3. **CNS 字库里也没有的笔画符号与部首变体**（`㇆` `㇉` `龹`）——笔顺列手工写死。

本文件的读音是「额外」的：字表里本来就有的读音照旧可用。

符号表的中文词只收两个字以上的：单字（上、左、角）一打就跟符号，太吵；单字本身就是符号读法的（度 → °）例外，列在 `src/emoji/builtin.rs` 测试的白名单里。CLDR 主表在 `assets/emoji/`。

## 五笔字根图

`roots86.tsv` 与 `roots86-glyphs.tsv` 是五笔键位提示的 86 版字根图：哪个字根在哪个键上、每个字根的字形。字形表编进 Core，各平台画出来一样，不随系统字体变。

| 文件 | 来源 | 许可 |
| --- | --- | --- |
| `roots86.tsv`（字根表） | 青简整理：对照通行的 86 版字根图逐键转写，键名与口诀核对维基百科「五笔字型输入法」。每行写字根取自哪个字、保留哪几个轮廓、露出哪一块 | 与仓库相同（GPL-3.0-or-later） |
| `roots86-glyphs.tsv`（字形表） | 由字根表从思源黑体 SC Regular（Source Han Sans，© Adobe，<https://github.com/adobe-fonts/source-han-sans>）取轮廓生成的 SVG 路径 | SIL Open Font License 1.1（字形是字体的衍生物，沿用字体许可；不以「Source」为名发布） |

部件（虎字头、无尾鱼、革字底……）大多没有自己的字符，从常用字里取：字体把相连的笔画合成一个闭合轮廓，
`roots86.tsv` 第五列写保留哪几个轮廓（编号从 0 起，空为整字），连在一起的笔画再用 `@左 上 右 下` 裁出一块（字面方块 0–1000，y 向下）。

字体不进仓库。从上面的地址下载 `09_SourceHanSansSC.zip`，取出 `SourceHanSansSC-Regular.otf`，再跑：

```bash
cargo run --release -p qingjian-dict-convert -- --out-dir crates/qingjian-core/data wubi-roots <SourceHanSansSC-Regular.otf>
```

改字根表时第五列写 `?`，加 `--sheet <图.png>` 把这些字画成一张轮廓编号图（每个轮廓描色、标号，底下有 0–1000 的网格），照着填回编号与裁剪框。

五笔码表本身在词库仓库 qingjian-dicts（`official/schemes/wubi86/`），随包时由 dicts-fetch 取成 `data/generated/schemes/wubi86.qj`。
