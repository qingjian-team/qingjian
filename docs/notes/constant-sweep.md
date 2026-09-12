# 排序常数扫描（2026-09-12）

`sentence` 里个人 n-gram 的插值常数（λ 0.8 / K 8 / 封顶 0.5 / 三元折扣 0.75）与 `correction` 里的敲错代价（换位 5 / 邻键 5 / 多键 5.5 /
少键 5.5、个人折扣上限 3、整段纠错 5）此前只按量级定过，没有用数据验过。这次把它们做成引擎可设（`Engine::set_interpolation` /
`set_typo_costs`，CLI `--tune 名=值`），在冻结日志上扫了一遍。**结论：全部在平台区，一个都不改。**

## 尺子

- 冻结日志：2026-09-12 早上的 `input-log.jsonl` 副本，27516 行，可评上屏 12886 条（词 9241 / 整句 3376 / 英文 269）。
- 回放：`qingjian-cli --replay`，内存学习从零起，云联想关（配置副本 `[predict] enabled = false`），不带神经重排（每组 7 秒，四路并行）。
  最后两组带神经重排（`--neural data/model`，Metal，每组十分钟）复核。
- 指标：各来源首选命中数（报告行末的「命中数」，比百分比细）。
- 扫参前先确认重构零变化：改后二进制缺省参数的报告与改前逐行相同。
- 这把尺子的偏差见 `docs/plan/model-eval.md`：整句「选了」多半是当时基线自己的输出，天然偏向现值；本轮又发现回放要把原样上屏也走一遍
  `take_raw`，否则个人英文词学不到、英文首选低 10 个点（72.5% → 82.9%）。这一改在第四轮之后，所以下表前三轮的基线是 词 8033 / 整句 3043，
  第四轮起是 8032 / 3045。

## 单轴与组合（不带神经重排，相对缺省的命中数差）

| 设置 | 词 | 整句 |
|---|---|---|
| `cap=0.3` | +5 | -14 |
| `cap=0.45` | +5 | -6 |
| `cap=0.4` | +7 | -6 |
| `cap=0.55` | +0 | +0 |
| `cap=0.6` | +0 | -2 |
| `cap=0.7` | -9 | -33 |
| `cap=0.9` | -81 | -87 |
| `correction=3` | +2 | -36 |
| `correction=4.5` | +6 | -10 |
| `correction=4` | +5 | -21 |
| `correction=6` | -11 | -3 |
| `correction=8` | -37 | -24 |
| `discount=0.5` | +15 | -10 |
| `discount=0.9` | +1 | +2 |
| `extra=4,missing=4` | +14 | -81 |
| `extra=4.5,missing=4.5` | +7 | -65 |
| `extra=5,missing=5` | +3 | -10 |
| `extra=6.5,missing=6.5` | -8 | -7 |
| `extra=7,missing=7` | -19 | -16 |
| `k=16` | -1 | -2 |
| `k=2` | +0 | -8 |
| `k=32` | -9 | -8 |
| `k=4` | +1 | -3 |
| `lambda=0.5` | +0 | -7 |
| `lambda=0.65` | -2 | +4 |
| `lambda=0.7,transpose=6,substitute=6,extra=6,missing=6` | -7 | +15 |
| `lambda=0.75,substitute=5.5` | +2 | +18 |
| `lambda=0.75,substitute=6` | +3 | +9 |
| `lambda=0.75,transpose=5.5,substitute=5.5,extra=5.5,missing=5.5` | +2 | +18 |
| `lambda=0.75,transpose=6,substitute=6,extra=6,missing=6` | -2 | +14 |
| `lambda=0.75` | +3 | +10 |
| `lambda=0.7` | -1 | +10 |
| `lambda=0.9` | +0 | -30 |
| `lambda=1.0` | -59 | -125 |
| `substitute=5.5,extra=6,missing=6` | -5 | +17 |
| `substitute=5.5` | +0 | +13 |
| `transpose=3.5,substitute=3.5` | +4 | -81 |
| `transpose=4,substitute=4` | +1 | -40 |
| `transpose=4.5,substitute=4.5` | +2 | -11 |
| `transpose=5,substitute=6` | +1 | +8 |
| `transpose=5.5,substitute=5.5,extra=5.5,missing=5.5` | +0 | +13 |
| `transpose=5.5,substitute=5.5` | +0 | +13 |
| `transpose=6,substitute=5` | +0 | +0 |
| `transpose=6,substitute=6,discount=0.9` | -1 | +5 |
| `transpose=6,substitute=6,extra=6,missing=6` | -3 | +14 |
| `transpose=6,substitute=6,lambda=0.65,discount=0.9` | -1 | +0 |
| `transpose=6,substitute=6,lambda=0.65` | -1 | +2 |
| `transpose=6,substitute=6` | +1 | +8 |
| `transpose=6.5,substitute=6.5` | +2 | +2 |
| `transpose=7,substitute=7` | -2 | -3 |
| `typo-cap=1` | -2 | -2 |
| `typo-cap=2` | +0 | +0 |
| `typo-cap=4` | +0 | +0 |

读法：

- λ：0.7–0.75 比 0.8 整句多 10 条（0.3 个点），0.9 起掉，1.0（不要个人一元）掉 125 条。K 从 2 到 32 全平。封顶 0.4–0.6 平，0.7 起掉，
  0.9 掉 80 多条：个人数据压死静态模型是真的坏。三元折扣 0.5 词 +15 / 整句 −10，0.9 平。
- 敲错代价：邻键（substitute）是唯一有反应的轴，5 → 5.5 整句 +13、6 +8、6.5 +2；换位单独动没变化（`transpose=6` 与缺省逐条相同）。
  多键 / 少键往便宜走整句掉得快（4 掉 81 条）、词略涨，5.5 就是峰。折扣上限 1 / 2 / 3 / 4 几乎没差：回放从零学，个人敲错表攒不到几次，
  这一项在回放里不可辨。整段纠错代价 5 是峰（3 掉 36、8 掉 24）。
- 最好的组合 `lambda=0.75,substitute=5.5`：词 +2 / 整句 +18（0.5 个点）。

## 带神经重排复核

壳里现在模型是开着的，插值与代价都在重排之前起作用，最终要看重排之后的数字：

| 设置 | 词 | 整句 | 英文 |
|---|---|---|---|
| 缺省 | 8085 | 3018 | 223 |
| `lambda=0.75,substitute=5.5` | 8080 | 3022 | 223 |

差 −5 / +4，是噪声。不带重排时的 +18 也只有 0.5 个点、在尺子偏差之内，所以不改。

顺带：神经重排在这把尺子上词 +53、整句 −27——整句掉是已知偏差（日志里的整句是旧基线接受的输出），整句质量看 `data/eval/sentences.tsv`。

## 没命中的到底是什么

常数不是瓶颈，那瓶颈在哪：

- 整句不在候选 328 条：16 条 青简（专名，词库没有）；294 条与现在首选同长、只是同音换字（184 条差一字、112 条差两字），
  这是上下文的活（神经重排、个人 n-gram），而且其中不少「选了」本身是当时接受的错句；18 条长度不同，是切分 / 简拼尾巴。
- 词没命中 1208 条：742 条是 5 键以上、当时从词候选里选的短语（导航栏 / 候选框 / 对齐 / 我绷不住了），多半是词库缺词；
  最常重复的键 `ba`（45 条）看上下文、`wode`（34 条）是词库缺「我的」（todo 里已有）。
- 英文 46 条没命中里 33 条是排在第 2–4 位（hello → helloWorld、com → commit、down → download），个人英文词的加权可以再看；13 条不在候选是
  当时敲错的拼写（trasnformer）与个人词。

下一步按这个顺序更值：从日志挖词库缺词（短语 + 专名），个人 n-gram 与神经重排在同音换字上的净效果（`rescored` 字段攒够后看），
`retype` 事件攒够后再回头看敲错代价与折扣上限。

## 怎么复现

```bash
cp ~/Library/Application\ Support/Qingjian/input-log.jsonl /tmp/frozen.jsonl
# 配置副本里把 [predict] enabled 改成 false
cargo build --release -p qingjian-cli
printf '\nlambda=0.75\nsubstitute=5.5\n' > /tmp/settings.txt
tools/eval/sweep.sh /tmp/frozen.jsonl /tmp/config.replay.toml /tmp/settings.txt /tmp/sweep.tsv
```

同一份冻结日志、同一个二进制才能比；日志是活的，改了尺子（比如原样上屏进学习）要连缺省一起重跑。
