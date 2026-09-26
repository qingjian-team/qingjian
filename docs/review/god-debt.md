# 上帝对象欠账台账

由 `scripts/gates/god_debt.py --write` 生成，**不要手改数字**——`--check` 会拿它与 `god-baseline.json` 对账，改了就红（想让数字变小只能真的去拆）。

<!-- debt-total: 6598 -->

- 阈值：文件 ≤ 800 行 / 最长函数 ≤ 100 行 / 最大类型 ≤ 20 成员（同 `scripts/gates/god.gate.json`）
- 欠账文件 45 个，总欠账 6598（加权：行数×1 + 函数×3 + 成员×2——函数最难读，权重最高）
- 基线在册 712 个文件；棘轮（god_gate）保证每项只准减 ⇒ 总欠账只准减
- 另有**跨文件上帝类型** 6 个 / 欠 555（见下表第二张；总欠账 = 6598 + 555 = 7153）

## 认领与清零

拆一块就在 `.agents/claims/<agent>.json` 里认领对应文件（避免几个智能体同时动一处），拆完跑 `python -X utf8 scripts/gates/gate.py --write` 重记基线，再跑 `--write` 更新本页。
三项硬阈（`god.gate.json` 的 `*_hard_threshold`）**全部翻 true 之后，欠账即清零**。

| # | 文件 | 欠账 | 明细 | 认领 |
| - | --- | --- | --- | --- |
| 1 | `apps/macos/src/host/settings.rs` | 960 | 最长函数行数 420→≤100（超 320） |  |
| 2 | `tools/dict-convert/src/lexicon/mod.rs` | 660 | 最长函数行数 320→≤100（超 220） |  |
| 3 | `apps/windows/settings/src/panel/component.rs` | 522 | 最长函数行数 274→≤100（超 174） |  |
| 4 | `crates/qingjian-core/src/engine/query/mod.rs` | 480 | 最长函数行数 250→≤100（超 150）、最大类型成员数 35→≤20（超 15） |  |
| 5 | `apps/linux/server/src/ipc/connection.rs` | 381 | 最长函数行数 227→≤100（超 127） |  |
| 6 | `crates/qingjian-core/src/engine/commit/mod.rs` | 342 | 最长函数行数 204→≤100（超 104）、最大类型成员数 35→≤20（超 15） |  |
| 7 | `apps/macos/src/imk/controller/text.rs` | 318 | 最长函数行数 206→≤100（超 106） |  |
| 8 | `apps/cli/src/main.rs` | 312 | 最长函数行数 204→≤100（超 104） |  |
| 9 | `apps/macos/src/host/init.rs` | 207 | 最长函数行数 169→≤100（超 69） |  |
| 10 | `tools/corpus/glossary_es.py` | 159 | 最长函数行数 153→≤100（超 53） |  |
| 11 | `apps/macos/src/preferences/pages/phrases/mod.rs` | 150 | 最长函数行数 150→≤100（超 50） |  |
| 12 | `crates/qingjian-core/src/zhuyin/syllable.rs` | 147 | 最长函数行数 149→≤100（超 49） |  |
| 13 | `crates/qingjian-core/src/engine/setup.rs` | 144 | 最大类型成员数 92→≤20（超 72） |  |
| 14 | `apps/macos/src/preferences/pages/general.rs` | 135 | 最长函数行数 145→≤100（超 45） |  |
| 15 | `tools/dict-convert/src/main.rs` | 126 | 最长函数行数 142→≤100（超 42） |  |
| 16 | `tools/dict-convert/src/bigram.rs` | 96 | 最长函数行数 132→≤100（超 32） |  |
| 17 | `crates/qingjian-core/src/engine/mod.rs` | 92 | 最大类型成员数 66→≤20（超 46） |  |
| 18 | `apps/linux/server/src/dispatch/linux.rs` | 87 | 最长函数行数 129→≤100（超 29） |  |
| 19 | `apps/macos/src/preferences/pages/shortcuts.rs` | 84 | 最长函数行数 128→≤100（超 28） |  |
| 20 | `crates/qingjian-core/src/sentence/viterbi.rs` | 78 | 最长函数行数 126→≤100（超 26） |  |
| 21 | `apps/macos/src/preferences/window.rs` | 71 | 最长函数行数 123→≤100（超 23）、最大类型成员数 21→≤20（超 1） |  |
| 22 | `apps/cli/src/replay/mod.rs` | 48 | 最长函数行数 116→≤100（超 16） |  |
| 23 | `apps/windows/tsf/src/com/service/key_sink.rs` | 48 | 最长函数行数 116→≤100（超 16） |  |
| 24 | `apps/macos/src/host/mod.rs` | 46 | 最大类型成员数 43→≤20（超 23） |  |
| 25 | `crates/qingjian-render/examples/preview.rs` | 39 | 最长函数行数 113→≤100（超 13） |  |
| 26 | `crates/qingjian-dictionary/src/dictionary/mod.rs` | 38 | 最大类型成员数 39→≤20（超 19） |  |
| 27 | `crates/qingjian-render/src/renderer/mod.rs` | 36 | 最大类型成员数 38→≤20（超 18） |  |
| 28 | `apps/windows/server/src/dispatch/message.rs` | 30 | 最长函数行数 110→≤100（超 10） |  |
| 29 | `apps/windows/settings/src/panel/pages/general.rs` | 30 | 最长函数行数 110→≤100（超 10） |  |
| 30 | `apps/macos/src/candidates/view/mod.rs` | 26 | 最大类型成员数 33→≤20（超 13） |  |
| 31 | `crates/qingjian-core/src/engine/composing.rs` | 26 | 最大类型成员数 33→≤20（超 13） |  |
| 32 | `apps/macos/src/preferences/pages/usage.rs` | 18 | 最长函数行数 106→≤100（超 6） |  |
| 33 | `apps/cli/src/args.rs` | 16 | 最大类型成员数 28→≤20（超 8） |  |
| 34 | `crates/qingjian-platform/src/config/general.rs` | 16 | 最大类型成员数 28→≤20（超 8） |  |
| 35 | `apps/windows/server/src/main.rs` | 15 | 最长函数行数 105→≤100（超 5） |  |
| 36 | `crates/qingjian-core/src/sentence/user_ngram.rs` | 14 | 最大类型成员数 27→≤20（超 7） |  |
| 37 | `apps/windows/server/src/dispatch/config.rs` | 10 | 最大类型成员数 25→≤20（超 5） |  |
| 38 | `apps/windows/server/src/dispatch/mod.rs` | 10 | 最大类型成员数 25→≤20（超 5） |  |
| 39 | `crates/qingjian-render/src/canvas.rs` | 8 | 最大类型成员数 24→≤20（超 4） |  |
| 40 | `crates/qingjian-core/src/composition.rs` | 4 | 最大类型成员数 22→≤20（超 2） |  |

（另有 5 个欠账文件未列出，跑 `--write` 前的完整清单见命令输出）

## 跨文件上帝类型（`type_span_gate.py`）

按类型名聚合 `impl` 块：`methods > 40` 或 `files > 8` 即欠账（加权 方法×2 + 文件×5——职责发散比单纯方法多更难改）。这类**每个文件都很小**，按文件量规模的门抓不到，只有聚合才看得见。

| # | 类型（crate\|类型名） | 欠账 | 方法 | 散在文件 | 认领 |
| - | --- | --- | --- | --- | --- |
| 1 | `crates/qingjian-core|Engine` | 374 | 207（超 167） | 16（超 8） |  |
| 2 | `apps/windows|Router` | 108 | 84（超 44） | 12（超 4） |  |
| 3 | `apps/linux|Router` | 31 | 53（超 13） | 9（超 1） |  |
| 4 | `crates/qingjian-learning|FrequencyLearner` | 22 | 51（超 11） | 3（超 0） |  |
| 5 | `apps/macos|Host` | 10 | 45（超 5） | 7（超 0） |  |
| 6 | `apps/windows|TextService_Impl` | 10 | 45（超 5） | 7（超 0） |  |
