# 拆字表

问字模式按部件读音找字（`umumu` → 林）的源数据：`chaizi-jt.txt` 一行一个字，后面是一到六种拆法，Tab 分隔，拆法里的部件用空格分隔（`谢\t言 射\t讠 射`）。

## 数据来源

| 文件 | 来源 | 许可 |
| --- | --- | --- |
| `chaizi-jt.txt` | [開放詞典網](http://kaifangcidian.com/) 的漢語拆字字典簡體版，[kfcd/chaizi](https://github.com/kfcd/chaizi) 提交 `e177ab54`（2017-08-31） | [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/)，原文见 `LICENSE` |

原样收录，未作改动。

## 生成

`cargo run -p qingjian-dict-convert -- --out-dir crates/qingjian-core/data chaizi` 读这张表与 `assets/lexicon/dict.tsv`（部件与字的读音、字频），
写 `crates/qingjian-core/data/chaizi.tsv`（编进 Core）。部件读不出来（`㇆` 这类笔画符号）的那种拆法跳过。
