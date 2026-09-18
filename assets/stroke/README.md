# 笔画表

随包笔画码表的源数据：`字\t序列`，序列按大陆笔顺，**1 横 / 2 竖 / 3 撇 / 5 折 / n 点与捺**
（CNS 的 `4` 同时涵盖点与捺，按设计文档「点捺合并为 n」写成 n；提在 CNS 里多数已记 1，氵的第三笔记 4，由覆盖表改回 1）。
单字取码「前 4 笔 + 末笔」、词组每字首笔，见 [docs/design/aux-code.md](../../docs/design/aux-code.md) 的「原生表：只带笔画」。

## 数据来源

| 用途 | 来源 | 许可 | 是否入库 |
| --- | --- | --- | --- |
| 字形来源（笔顺序列） | CNS11643 中文標準交換碼全字庫「筆順資料」`CNS_strokes_sequence.txt` | 政府資料開放授權條款第 1 版 **或** OFL-1.1（二选一，可再分发含商用，**需署名**） | 原始 zip 不入库（`data/cns/`，gitignore），只留生成产物 |
| CNS→Unicode 映射 | 同一数据集的 `MapingTables.zip`（`CNS2UNICODE_Unicode*.txt` 4 个平面） | 同上 | 不入库（`data/cns/`） |
| 自洽过滤 | 同一数据集的 `CNS_stroke.txt`（筆畫數） | 同上 | 不入库（`data/cns/`） |
| 抽样对照（**开发期**） | Make Me a Hanzi / hanzi-writer-data 2.0.1 每字 `strokes` 条数（PRC 笔顺） | Arphic Public License；**不随包分发**，只留「字 + 笔画数」这类事实性结论 | `tools/dict-convert/testdata/prc-counts-l1.tsv`（一级字 3,500 行） |

数据集页 <https://data.gov.tw/dataset/5961>，下载 <https://www.cns11643.gov.tw/opendata/Properties.zip> 与
<https://www.cns11643.gov.tw/opendata/MapingTables.zip>（解到 `data/cns/`）；对照源 `hanzi-writer-data-2.0.1.tgz`（npm registry）解到 `data/mmh/`。
选型与许可依据见 [docs/design/aux-code.md](../../docs/design/aux-code.md)「原生表：只带笔画」与 wayfinder 的
t07（数据源查证）/ t09（归一化决议）两张票：**CNS 作字形来源 + 大陆序覆盖表**，MMH 只在开发期做对照。

## 文件

| 文件 | 内容 |
| --- | --- |
| `prc-rules.tsv` | 大陆序覆盖表：`rule`（部件重写）/ `skip`（例外字）/ `char`（整字补录）三类行，格式见文件头 |
| `tools/dict-convert/testdata/prc-counts-l1.tsv` | 一级字大陆笔画数对照表（抽样对照用；来源 MMH，见上表） |
| `residual-whitelist.tsv` | 残留差异白名单：抽样里已知且接受的差异（7 字），白名单之外一处不符即验收失败 |

## 生成与验收

```bash
# 生成（缺省写 data/generated/codes/stroke.tsv）
cargo run --release -p qingjian-dict-convert -- stroke --cns-count data/cns/CNS_stroke.txt --verify
```

`--verify` 按「一级字表每 12 字取 1」（291 字）逐字比对大陆笔画数：不符的字必须都在白名单里，否则退出码非 0。

随包时再算成码表（缺省读 `data/generated/codes/stroke.tsv` 与 `data/generated/dict.qj`，写 `data/generated/codes/stroke.qj`）：

```bash
cargo run --release -p qingjian-dict-convert -- pack codes
```

按设计文档的取码规则算码（单字「前 4 笔 + 末笔」、词组每字首笔，缺字的词跳过并计入统计）；元数据缺省写明
名称「笔画」、许可 `OFL-1.1`、署名「CNS11643 全字庫筆順資料（中華民國數位發展部）」与数据集页来源，
可用 `--name` / `--license` / `--attribution` / `--source` / `--data-version` 覆盖（数据版本缺省取笔画表日期）。
产物随包只带生成结果，原始 zip 与对照源都不入库。

### 验收记录（2026-09-15）

```
已读大陆序覆盖表 rules=8 overrides=1
已读 CNS→Unicode 对照表 entries=104680
已读筆順資料 entries=94919
已读筆畫數 entries=97361
已读字表白名单 chars=8105
已生成笔画表 out=data/generated/codes/stroke.tsv entries=7990 dropped_no_sequence=111
             dropped_inconsistent=4 average_strokes=10.92 size_kb=127 elapsed_ms=579
抽样对照完成 sampled=291 reference_entries=3500 whitelisted=7 unmatched=0
```

- **抽样对照**：291 字里 7 字不符，全部在白名单内，白名单之外 **0 条**（`cargo test -p qingjian-dict-convert` 的
  `stroke::tests` 另外覆盖规则解析、锚点语义与对照/抽样逻辑）。
- **产物**：7,990 行 / 127,283 字节（平均 10.9 画）。设计文档估的 40–80 KB 偏小：那是按「前 4 笔 + 末笔」的 5 码估的，
  本表是全笔顺序列（`字\t[1-5n]+`）。
- **覆盖面**：通用规范字表 8,105 字里 7,990 字有码。CNS 对照表里没有的 112 字里，`闹` 由 `char` 行人工补录，
  其余 111 字绝大多数是三级字表里的扩展 B 区字（𬉼 这类），另有 鿍 / 鿎 / 鿏（U+9FCD–9FCF）；
  另有 4 字（诞 铼 煺 藓）笔顺序列与 CNS 筆畫數差超过 1，被自洽过滤丢掉。
- **已知范围（诊断，不入白名单口径）**：全量 6,861 个有对照的字里 **241 字（3.5%）笔画数与大陆规范不符**，
  是 CNS 台湾标准与大陆规范在整字形 / 部件层面的结构性差异（例：瓦 5/4、卸 8/9、修 10/9、粤 13/12）。
  设计文档「差异集中在艹 / 辶 / 阝」只对了一部分：覆盖表把不符从 926 处压到 241 处（修掉 685 处成片差异），
  剩下的（及 / 巨 / 之 / 与 / 母 / 书 / 我 的字形与笔顺归类差异）不能由模式替换从 CNS 序列推出，
  要逐字人工核；抽样白名单只登记落在抽样里的 7 个（`--stride` 越小抽样越密，白名单要同步增补）。
- **已知范围之二：首笔不保证对**（2026-09-16 补测，诊断口径）。上一节的抽样只比**笔画数**，比不出「笔数对、
  顺序不同」的偏差。用开发期对照源（`data/mmh/`，hanzi-writer-data 的 medians）对全表首笔方向做了一次粗筛：
  6,862 个可比字里 58 处（0.8%）方向与我们的首笔类型对不上，其中一部分是提 / 竖提 / 折这类「末端转向」被误判，
  但 **丰 / 耒 / 皮 一族的首笔确实可疑**（丰 我们的序列 `311`：首笔记成撇，大陆规范是横），而首笔错了，
  含这个字的每个词的码都跟着错。示例：发 `535nn`（首笔记成折，对照源的几何方向接近横）。
  复现：`node .scratch/first-stroke-audit.js`（仓库外工具，脚本未入库）；修法是把这类成片的字形差异补进
  `prc-rules.tsv`（或加一张「首笔覆盖表」），再重跑 `stroke` + `pack codes`。

## 随包时的许可合规

- 数据目录与「关于」页署名：**資料來源「CNS11643 中文標準交換碼全字庫」（數位發展部）**，并声明本表为派生
  （筆順資料表格 → 笔画码表）与修改内容（大陆序覆盖表）；
- 随包数据目录放 `LICENSE-CNS11643`（政府資料開放授權條款第 1 版全文，或改选 OFL-1.1 全文——数据集允许二选一）。

*来源：wayfinder 票 t17（stroke 工具与笔画表生成）与 spec 卷 I 第 8 章的 stroke 契约（在仓库外的 .wayfinder 工作区）。*
