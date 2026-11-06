# 拆字表

问字模式按部件找字（`umumu` → 林、`uheszh` → 和）的源数据：一行一个字，Tab 分隔，
后面是一到六种拆法，拆法里的部件用空格分隔（`谢\t言 射\t讠 射`）。简体一份、繁体一份。

## 数据来源

| 文件 | 来源 | 许可 |
| --- | --- | --- |
| `chaizi-jt.txt` | [開放詞典網](http://kaifangcidian.com/) 的漢語拆字字典**簡體版**，[kfcd/chaizi](https://github.com/kfcd/chaizi) 提交 `e177ab54`（2017-08-31） | [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/)，原文见 `LICENSE` |
| `chaizi-ft.txt` | 同一个仓库的**繁体版**（`chaizi-ft.txt`） | 同上 |
| `fanjian_suoyin.txt` | 同一个仓库的繁简索引（`繁体\t简体`，一行一对） | 同上 |
| `parts.tsv` | 本仓库手工维护：部件别名的读音与笔顺，见下 | 同本仓库（GPL-3.0-or-later） |

原样收录，未作改动。

## 繁体怎么进来

繁体表里「简体表没有的字」按 `fanjian_suoyin.txt` 落到简体主键（`謝` → `谢`），拆法并进同一个字——
于是打台北字形的拆法（`辶 言`，`uzouyan`）也能找到简体字，输出侧照旧由 Core 的 `Traditional` 在出口转。
索引里没有简体对应的字（繁简索引写成 `□` 占位的那种）保留原形，不丢。

`□` 这类方框符号是拆字字典里的占位部件，不是字：字列直接跳过，只作为部件留在拆法里。

## parts.tsv：部件别名

三列 `部件<TAB>别名读音（空格分隔）<TAB>笔顺（空格分隔的 h/s/p/n/z）`，只收三类：

1. **俗名与字表读音不同的**——`囗` 的字表读音是 `wei`，可用户按 `kou` 打（`ukoumu` → 困）；
2. **通用规范字表与繁体拆字表都没覆盖的字**（`馬` `長` `糹` 这些）——CNS 有笔顺，
   把本文件当 `stroke --filter` 的一张表传进去就能补上，笔顺列留空；
3. **CNS 字库里也没有的笔画符号与部首变体**（`㇆` `㇉` `龹`）——笔顺列手工写死。

本文件的读音是「额外」的：字表里本来就有的读音照旧可用。

## 生成

```bash
# 拆字表：简体 + 繁体 + 繁简索引 + 词库读音 → crates/qingjian-core/data/chaizi.tsv
cargo run -p qingjian-dict-convert -- --out-dir crates/qingjian-core/data chaizi

# 笔顺表（键位形式，给问字模式按笔画找字）：把 parts.tsv 也当一张字表传进去，部件的笔顺才补得上
cargo run -p qingjian-dict-convert -- --out-dir crates/qingjian-core/data stroke \
  --cns-count data/cns/CNS_stroke.txt \
  --filter assets/lexicon/01_characters/standard_8105.tsv,assets/chaizi/chaizi-ft.txt,assets/chaizi/parts.tsv \
  --output crates/qingjian-core/data/stroke.tsv --keys
```

产物都是生成物，随包前由 `cargo run -p qingjian-dict-convert -- pack codes` 另算码表（辅码态用，与本目录无关）。
