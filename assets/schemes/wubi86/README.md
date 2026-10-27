# 五笔（86）

随包的形码方案，与用户放进 `schemes/` 的方案同一个格式：`scheme.toml` 写名字与按键规则，
`table.tsv` 是码表（`词\t编码\t词频`，`qingjian-dictionary::CodeTable` 读，按编码前缀查）。
`roots86.tsv` 与 `roots86-glyphs.tsv`：五笔键位提示的 86 版字根图（哪个字根在哪个键上、每个字根的字形），编进 Core，不随方案目录拷。

| 文件 | 来源 | 许可 |
| --- | --- | --- |
| `roots86.tsv`（字根表） | 青简整理：对照通行的 86 版字根图逐键转写，键名与口诀核对维基百科「五笔字型输入法」。每行写字根取自哪个字、保留哪几个轮廓、露出哪一块 | 与仓库相同（GPL-3.0-or-later） |
| `roots86-glyphs.tsv`（字形表，产品数据） | 由字根表从思源黑体 SC Regular（Source Han Sans，© Adobe，<https://github.com/adobe-fonts/source-han-sans>）取轮廓生成的 SVG 路径 | SIL Open Font License 1.1（字形是字体的衍生物，沿用字体许可；不以「Source」为名发布） |
| `wubi86_jidian.dict.yaml`（上游源文件，8.9 万条，不进仓库） | 86 五笔极点码表，<https://github.com/sxjudya/rime-wubi86-jidian>（`version: "4.3"`） | Apache-2.0，原文见同目录 `LICENSE` |
| `table.tsv`（产品数据） | 由上面那份转换而来 | 编码来自上游（Apache-2.0）；词频是青简自己统计的 |

上游还有 `wubi86_jidian_extra.dict.yaml`（扩展词）与 `wubi86_jidian_extra_district.dict.yaml`（行政区域），**暂未并入**：
极点方案的 `import_tables` 收了前者，以后按需要再加。

同一单字母编码的词按上游码表顺序保留，保证一级简码排在其他同码单字之前；其余编码按青简词频排序。

## 词频不是码表自带的权重

码表第二列（Rime 的 `weight`）是码表顺序，不是语料词频。直接拿来排序，同一个词在五笔下和在拼音下会排得不一样，
释义兜底与生词识别也对不上。所以转换时**按词面**从青简词库回填词频：词库里有的用语料词频（与拼音方案同一把尺子），
没有的给 `UNKNOWN_FREQUENCY = 1`。

## 重新生成

先从上游仓库下载 `wubi86_jidian.dict.yaml`（放哪都行，下面以 `data/raw/` 为例），再跑（`--frequency` 建议给基础词库 + 全部领域词库，覆盖率更高）：

```bash
cargo run --release -p qingjian-dict-convert -- --out-dir assets/schemes/wubi86 wubi \
    data/raw/wubi86_jidian.dict.yaml \
    --frequency assets/lexicon/dict.tsv assets/lexicon/dicts/*.tsv
```

命中率（2026-09-16，基础词库 + 领域词库）：8.9 万条里 5.67 万条拿到语料词频，3.25 万条是兜底。
兜底的那部分主要是《通用规范汉字表》之外的罕见字（菚 / 匞 / 葚 这类）与青简词库没收的成语、词组；
它们在候选里排在已知词后面，只有在同编码没有别的选择时才出现。

## 字根图的字形

部件（虎字头、无尾鱼、革字底……）大多没有自己的字符，从常用字里取：字体把相连的笔画合成一个闭合轮廓，
`roots86.tsv` 第五列写保留哪几个轮廓（编号从 0 起，空为整字），连在一起的笔画再用 `@左 上 右 下` 裁出一块（字面方块 0–1000，y 向下）。
字形一律取自同一个字体，各平台画出来一样，不随系统字体变。

字体不进仓库。从上面的地址下载 `09_SourceHanSansSC.zip`，取出 `SourceHanSansSC-Regular.otf`，再跑：

```bash
cargo run --release -p qingjian-dict-convert -- --out-dir assets/schemes/wubi86 wubi-roots <SourceHanSansSC-Regular.otf>
```

改字根表时第五列写 `?`，加 `--sheet <图.png>` 把这些字画成一张轮廓编号图（每个轮廓描色、标号，底下有 0–1000 的网格），照着填回编号与裁剪框。
