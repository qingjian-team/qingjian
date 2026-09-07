# 英文词库

用于英文补全、中英混输和拼写提示的英文文本词库。收录常用词、扩展词、专名与缩写、词形变化、技术词及误拼对照，保留原始大小写和来源。

## 文件

| 文件 | 内容 |
| --- | --- |
| `00_英文补全总表` | 通用英文与技术英文的去重合集 |
| `01_常用英文词` | ESDB常见度级别不超过50的小写词形 |
| `02_扩展英文词` | ESDB级别51–60的小写词形，与常用词文件不重叠 |
| `03_专名与缩写` | ESDB收录的含大写字母词形，包括专名、缩写及其变化形式 |
| `04_inflections.tsv` | 原形与屈折词形的对应关系，包括复数、时态、比较级等 |
| `05_tech` | 软件与编程、软件工具、网络通信、网络服务、计算机缩写 |
| `06_misspellings.tsv` | 误拼到一个或多个建议词形的对照 |
| `sources` | 上游许可证、来源清单、词性代码和分类统计 |

TXT为一行一词，TSV为UTF-8、Tab分隔、有表头。总表与各分类文件提供两种读取入口，内容有包含关系。

## 字段

列名统一用英文：`word` 词条、`key` 检索键、`frequency_level` 常见度级别、`spelling_region` 拼写地区、`pos_code` 词性代码、
`lemma` 原形、`form` 词形、`form_note` 词形说明、`tech_domain` / `domain` 技术领域、`misspelling` 误拼、`suggestion` 建议词形、`source` 来源。

检索键是词条的casefold形式，展示词条保留大小写。ESDB常见度级别越低，词表覆盖层级越基础；该字段不是实测词频或概率。技术词无该值时留空。拼写地区包括通用、美式、英式-ise和英式-ize，英美有效拼写均保留。英文数据不包含中文释义。

词形变化中的词性代码定义见`sources/esdb_pos_codes.tsv`。一个词形可对应多个原形或词性。误拼对照允许一对多，不表达确定替换或自动纠正；有效英文词形与上游允许项已从误拼端排除，所有建议词形均存在于补全总表。

## 数据范围

ESDB使用size≤60、variant_level≤1，选择通用/美国/英国拼写及地区，排除特殊category。单词表保留字母及词内撇号；复合短语和带点缩写不包含在通用词表中。技术词另保留字母开头的字母数字词以及部分技术符号。数据是有限词表，不能覆盖所有有效专名、拼写变化或误拼。

## sources

- [English Speller Database](https://github.com/en-wl/wordlist)：常用、扩展、专名及词形数据；MIT-like宽松授权，原始Copyright全文随包保留，其中包含词性元数据涉及的WordNet声明。
- [CSpell software-terms](https://github.com/streetsidesoftware/cspell-dicts/tree/main/dictionaries/software-terms)：技术词；MIT。
- [typos](https://github.com/crate-ci/typos)：误拼对照；上游MIT或Apache-2.0双许可，本包采用MIT。

保留随附版权及许可声明。整理包含筛选、去重、字段转换及纠错目标词形过滤。原始文件哈希和获取地址见来源清单。
