# 整句词图按上下文放词：实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 整句词图的格子在一元词频前 6 个之外，再放进「读音占比够高、且在某个前文下比首段都可能」的词，让 `jishimu` → 几十亩。

**Architecture:** 只改 Core 的 `sentence` 与 `qingjian-dictionary`。`Dictionary` 懒建「词文本 → 全部读音词频和」的表，`span_candidates` 多留到 `SPAN_POOL` 个并算好读音占比，
`convert_paths` 主循环对首段之后的词按前驱现算前文抬举，过关的才进节点。打分、重排、束宽都不动。设计见 [spec](../specs/2026-10-05-sentence-context-admission-design.md)。

**Tech Stack:** Rust（workspace 现有版本），`apps/cli` 的 `--eval-text` / `--replay` / `--typing` 当尺子。

**约定提醒（见 `docs/contributing.md`）：** 一个类型一个文件、新文件 `//!` 头、注释中文且只写约束与原因、不写装饰性分隔注释、
文件不超过 800 行（测试超过 200 行搬 `tests.rs`）、导入写精确路径。提交信息 Conventional Commits，范围 `core` / `dictionary`，
末尾加 `Co-Authored-By`。每个任务提交前 `cargo fmt --all` 与 `cargo clippy --all-targets -- -D warnings`。

---

## 文件结构

| 路径 | 职责 |
| --- | --- |
| `crates/qingjian-dictionary/src/dictionary/text_totals.rs` | 新增 `TextTotals`：词文本 → 本词库全部读音词频和 |
| `crates/qingjian-dictionary/src/dictionary/mod.rs` | `Dictionary` 加 `text_totals: OnceLock<TextTotals>` 字段与 `text_frequency()` |
| `crates/qingjian-core/src/sentence/mod.rs` | 新常数 `SPAN_POOL`、`MIN_READING_SHARE` |
| `crates/qingjian-core/src/sentence/span/word.rs` | `SpanWord` 加 `reading_share` |
| `crates/qingjian-core/src/sentence/viterbi.rs` → `viterbi/mod.rs` | 现有实现原样搬家；主循环接候补段 |
| `crates/qingjian-core/src/sentence/viterbi/tests.rs` | 现有测试搬过来（已超 200 行）+ 新测试 |
| `crates/qingjian-core/src/sentence/viterbi/admission.rs` | 自由函数：候补词能不能进格子 |
| `docs/notes/crate-notes.md` | 记新常数与评测数字 |

---

## Task 0：准备尺子与基线

**Files:** 无代码改动（`data/` 整个 gitignore）。

- [ ] **Step 1：备好产品数据与评测集**

```bash
R="$HOME/Library/Input Methods/Sujian.app/Contents/Resources"
mkdir -p data/generated data/eval
ln -sf "$R/lm.qj" data/generated/lm.qj
ln -sf "$R/dict.qj" data/generated/dict.qj
cp ../qingjian-local/data/eval/sentences.tsv ../qingjian-local/data/eval/input-log-2026-10-04.jsonl data/eval/
```

- [ ] **Step 2：编出基线二进制并记数字**

```bash
cargo build --release -p qingjian-cli && cp target/release/qingjian-cli "$TMPDIR/qj-base"
M="$HOME/Library/Input Methods/Sujian.app/Contents/Resources/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm"
QINGJIAN_API_KEY=x "$TMPDIR/qj-base" --eval-text data/eval/sentences.tsv --eval-details "$TMPDIR/base.jsonl" 2>&1 | grep '^句子'
QINGJIAN_API_KEY=x "$TMPDIR/qj-base" --neural "$M" --eval-text data/eval/sentences.tsv 2>&1 | grep '^句子'
QINGJIAN_API_KEY=x "$TMPDIR/qj-base" --replay data/eval/input-log-2026-10-04.jsonl 2>&1 | tail -5
QINGJIAN_API_KEY=x "$TMPDIR/qj-base" --typing woxiangzaishurudetongshijizhuyigecihuiranhoubatajiaruciku 2>&1 | tail -1
```

预期（2026-10-05 测过）：冷启动首选 32.7% / 前三 38.3% / 字准确率 76.8%；接通变 37.5% / 40.1% / 79.1%；逐键平均约 0.64 ms。
把四组输出贴进 PR 描述。`QINGJIAN_API_KEY=x` 只是让读到 `[predict] enabled` 的配置不报错，评测不走云。

---

## Task 1：`viterbi.rs` 搬成目录（不改行为）

**Files:**
- Move: `crates/qingjian-core/src/sentence/viterbi.rs` → `crates/qingjian-core/src/sentence/viterbi/mod.rs`
- Create: `crates/qingjian-core/src/sentence/viterbi/tests.rs`

- [ ] **Step 1：搬家**

```bash
mkdir crates/qingjian-core/src/sentence/viterbi
git mv crates/qingjian-core/src/sentence/viterbi.rs crates/qingjian-core/src/sentence/viterbi/mod.rs
```

- [ ] **Step 2：测试模块搬进 `tests.rs`**

把 `mod.rs` 里 `#[cfg(test)] mod tests { … }` 的**内容**原样剪到 `viterbi/tests.rs`（文件头加 `//! 词图 Viterbi 的单元测试。`，保留 `use super::*;`），
`mod.rs` 末尾只留：

```rust
#[cfg(test)]
mod tests;
```

- [ ] **Step 3：跑测试确认没变**

```bash
cargo test -p qingjian-core sentence::viterbi
```

预期：与搬家前同样的测试全部通过。

- [ ] **Step 4：提交**

```bash
git add -A crates/qingjian-core/src/sentence/viterbi
git commit -m "refactor(core): viterbi 拆成目录，测试搬进 tests.rs"
```

---

## Task 2：`Dictionary::text_frequency`

**Files:**
- Create: `crates/qingjian-dictionary/src/dictionary/text_totals.rs`
- Modify: `crates/qingjian-dictionary/src/dictionary/mod.rs`（字段、两处 `Self { … }` 构造、新方法）
- Test: `crates/qingjian-dictionary/src/dictionary/tests.rs`

- [ ] **Step 1：写失败的测试**

`dictionary/tests.rs` 末尾加：

```rust
#[test]
fn text_frequency_sums_every_reading_of_a_text() {
    let dictionary = Dictionary::parse("和\the\t900\n和\thuo\t30\n亩\tmu\t10\n木\tmu\t300\n").unwrap();
    assert_eq!(dictionary.text_frequency("和"), 930);
    assert_eq!(dictionary.text_frequency("亩"), 10);
    assert_eq!(dictionary.text_frequency("没有的词"), 0);
}
```

- [ ] **Step 2：确认失败**

```bash
cargo test -p qingjian-dictionary text_frequency_sums_every_reading
```

预期：编译失败，`no method named text_frequency`。

- [ ] **Step 3：实现 `TextTotals`**

`dictionary/text_totals.rs`：

```rust
//! 词文本 → 本词库里这个词全部读音的词频之和。整句词图按它算读音占比，挡住多音字的冷门读音。

use std::collections::HashMap;

use crate::matching::Match;

#[derive(Debug, Default)]
pub(super) struct TextTotals {
    totals: HashMap<Box<str>, u64>,
}

impl TextTotals {
    pub(super) fn build<'a>(entries: impl Iterator<Item = Match<'a>>) -> Self {
        let mut totals: HashMap<Box<str>, u64> = HashMap::new();
        for entry in entries {
            *totals.entry(entry.text.into()).or_insert(0) += u64::from(entry.frequency);
        }
        Self { totals }
    }

    pub(super) fn get(&self, text: &str) -> u64 {
        self.totals.get(text).copied().unwrap_or(0)
    }
}
```

`dictionary/mod.rs`：

```rust
mod text_totals;

use std::sync::OnceLock;

use text_totals::TextTotals;
```

`Dictionary` 末尾加字段：

```rust
    /// 词文本 → 全部读音词频和，第一次用到时才建（不拖慢启动，也不改 `.qj` 格式）。
    text_totals: OnceLock<TextTotals>,
```

`assemble` 与 `open_qj` 里的 `Self { … }` 都补 `text_totals: OnceLock::new(),`。在 `total_frequency` 之后加：

```rust
    /// 这个词文本在本词库所有读音下的词频之和；没有这个词是 0。
    pub fn text_frequency(&self, text: &str) -> u64 {
        self.text_totals
            .get_or_init(|| TextTotals::build(self.entries()))
            .get(text)
    }
```

- [ ] **Step 4：跑测试**

```bash
cargo test -p qingjian-dictionary
```

预期：全部通过。

- [ ] **Step 5：提交**

```bash
git add crates/qingjian-dictionary/src/dictionary
git commit -m "feat(dictionary): 按词文本汇总全部读音的词频"
```

---

## Task 3：格子多留候补并带上读音占比

**Files:**
- Modify: `crates/qingjian-core/src/sentence/mod.rs`（常数、re-export 不变）
- Modify: `crates/qingjian-core/src/sentence/span/word.rs`
- Modify: `crates/qingjian-core/src/sentence/span/cache.rs:99`（测试里的 `SpanWord { … }` 补字段）
- Modify: `crates/qingjian-core/src/sentence/viterbi/mod.rs`（`span_candidates`）

这一步只多存数据，主循环仍然只用前 `SPAN_CANDIDATES` 个，行为不变。

- [ ] **Step 1：常数**

`sentence/mod.rs` 在 `SPAN_CANDIDATES` 后加：

```rust
/// 全拼格子最多查出几个词：前 [`SPAN_CANDIDATES`] 个无条件进词图，其余是候补，要前文抬举才进
/// （`ji shi mu` 的 亩 按词频排第 12，P(亩|十) 却比首段都高）。设计见 `docs/superpowers/specs/2026-10-05-sentence-context-admission-design.md`。
pub const SPAN_POOL: usize = 16;

/// 候补词的读音占比下限：这个读音的词频占该词全部读音之和的比例。二元模型按词文本计数、不分读音，
/// 冷门读音（和/huo、没/mo）会借常用读音的文本概率混进路径，占比够高的词文本概率才代表这个读音。
pub const MIN_READING_SHARE: f64 = 0.5;
```

- [ ] **Step 2：`SpanWord` 加字段**

```rust
    /// 这个读音的词频占该词全部读音词频之和的比例（0 到 1）；候补词按它挡冷门读音。
    pub reading_share: f64,
```

`span/cache.rs` 测试里构造 `SpanWord` 处补 `reading_share: 1.0,`。

- [ ] **Step 3：`span_candidates` 留到 `SPAN_POOL` 并算占比**

`truncate` 的全拼分支由 `SPAN_CANDIDATES` 改成 `SPAN_POOL`；`map` 里：

```rust
        .map(|(_, penalty, hit)| {
            let total: u64 = dictionaries.iter().map(|d| d.text_frequency(hit.text)).sum();
            SpanWord {
                text: hit.text.to_owned(),
                syllables: hit.syllables().map(str::to_owned).collect(),
                frequency: hit.frequency,
                penalty,
                reading_share: if total == 0 {
                    1.0
                } else {
                    f64::from(hit.frequency) / total as f64
                },
            }
        })
```

`use super::{…}` 里加 `SPAN_POOL`。

- [ ] **Step 4：主循环暂时只取首段**

`convert_paths` 主循环的 `for hit in hits.iter()` 换成只走首段（简拼格子整段都是首段）：

```rust
            let abbreviated = span.iter().any(|p| p.iter().any(|t| !t.complete));
            let head = if abbreviated { hits.len() } else { hits.len().min(SPAN_CANDIDATES) };
            for hit in &hits[..head] {
```

- [ ] **Step 5：确认行为不变**

```bash
cargo test -p qingjian-core
cargo build --release -p qingjian-cli
QINGJIAN_API_KEY=x target/release/qingjian-cli --eval-text data/eval/sentences.tsv 2>&1 | grep '^句子'
```

预期：测试全过；评测数字与 Task 0 基线完全一致（32.7% / 38.3% / 76.8%）。

- [ ] **Step 6：提交**

```bash
git add crates/qingjian-core/src/sentence
git commit -m "refactor(core): 整句格子多查候补词并记读音占比"
```

---

## Task 4：候补词按前文进格子

**Files:**
- Create: `crates/qingjian-core/src/sentence/viterbi/admission.rs`
- Modify: `crates/qingjian-core/src/sentence/viterbi/mod.rs`
- Test: `crates/qingjian-core/src/sentence/viterbi/tests.rs`

- [ ] **Step 1：写失败的测试**

`viterbi/tests.rs` 加：

```rust
/// `mu` 下有 7 个比 亩 常用的字，亩 落在首段之外；和 有冷门读音 huo。
const MU: &str = "几\tji\t70000\n即\tji\t30000\n十\tshi\t70000\n使\tshi\t40000\n即使\tji shi\t11000\n\
    木\tmu\t33000\n姆\tmu\t17000\n母\tmu\t14000\n穆\tmu\t8000\n目\tmu\t8000\n墓\tmu\t5000\n牧\tmu\t3000\n亩\tmu\t1000\n\
    或\thuo\t100000\n和\the\t900000\n和\thuo\t3000\n火\thuo\t50000\n活\thuo\t40000\n货\thuo\t30000\n获\thuo\t20000\n伙\thuo\t10000\n";

/// 只认 十 → 亩、我 → 和 两个强接续的假模型，其余都兜底。
struct MuModel;

impl LanguageModel for MuModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        match (previous, word) {
            (None, "几") => Some(-3.0),
            (Some("几"), "十") => Some(-1.0),
            (Some("十"), "亩") => Some(-1.0),
            (None, "我") => Some(-1.0),
            (Some("我"), "和") => Some(-1.0),
            _ => None,
        }
    }
}

fn with_mu_model(syllables: &[&str]) -> String {
    let dictionary = Dictionary::parse(&format!("{MU}我\two\t900000\n")).unwrap();
    convert(
        &[&dictionary],
        &complete(syllables),
        &MuModel,
        Personal::NONE,
        |_| 0,
        |_, _| 0.0,
        &mut SpanCache::default(),
    )
    .unwrap()
    .text
}

#[test]
fn context_admits_a_rare_single_reading_word() {
    assert_eq!(with_mu_model(&["ji", "shi", "mu"]), "几十亩");
}

#[test]
fn context_does_not_admit_a_rare_reading_of_a_polyphone() {
    // 和/huo 占 和 全部读音不到 1%：模型给 我 → 和 再高也不能拿来读 huo
    assert_ne!(with_mu_model(&["wo", "huo"]), "我和");
}

#[test]
fn rare_word_without_context_stays_out() {
    // 句首没有前文抬举，亩 进不了格子
    assert_ne!(with_mu_model(&["mu"]), "亩");
}
```

- [ ] **Step 2：确认失败**

```bash
cargo test -p qingjian-core sentence::viterbi::tests::context_admits
```

预期：`context_admits_a_rare_single_reading_word` 失败（实际是含 木 / 姆 的路径）；另外两条已经通过（首段之外的词现在一律进不来），保留作回归。

- [ ] **Step 3：实现 `admission.rs`**

```rust
//! 整句格子的候补词能不能进词图：读音占比够高，并且在某个前驱后面比首段里最好的词还可能。

use crate::sentence::{LanguageModel, MIN_READING_SHARE, SpanWord, fallback_log_prob};

/// 某个前驱后面，首段里最可能的词的静态 log 概率。`previous` 为 `None` 是句首。
pub(super) fn head_ceiling(
    head: &[SpanWord],
    previous: Option<&str>,
    model: &dyn LanguageModel,
    log_total: f64,
) -> f64 {
    head.iter()
        .map(|w| {
            model
                .log_prob(previous, &w.text)
                .unwrap_or_else(|| fallback_log_prob(w.frequency, log_total))
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

/// 候补词 `word` 能进词图：读音占比不低于 [`MIN_READING_SHARE`]，并且至少一个前驱后面它的静态 log 概率严格高于首段的最高值。
/// `previous_and_ceiling` 是每个前驱（`None` 为句首）与它对应的 [`head_ceiling`]。
/// 只看静态模型：个人常用的词已经靠 `seen` 进了首段。
pub(super) fn admits(
    word: &SpanWord,
    previous_and_ceiling: &[(Option<&str>, f64)],
    model: &dyn LanguageModel,
    log_total: f64,
) -> bool {
    if word.reading_share < MIN_READING_SHARE {
        return false;
    }
    let fallback = fallback_log_prob(word.frequency, log_total);
    previous_and_ceiling.iter().any(|&(previous, ceiling)| {
        model.log_prob(previous, &word.text).unwrap_or(fallback) > ceiling
    })
}
```

`viterbi/mod.rs` 加 `mod admission;`，主循环 Task 3 的 `head` 之后改成：

```rust
            let extras: Vec<&SpanWord> = if hits.len() > head {
                let ceilings: Vec<(Option<&str>, f64)> = nodes[start]
                    .iter()
                    .map(|p| {
                        let previous = (start > 0).then_some(p.text.as_str());
                        (previous, admission::head_ceiling(&hits[..head], previous, model, log_total))
                    })
                    .collect();
                hits[head..]
                    .iter()
                    .filter(|w| admission::admits(w, &ceilings, model, log_total))
                    .collect()
            } else {
                Vec::new()
            };
            for hit in hits[..head].iter().chain(extras) {
```

（`hits` 是从 `SpanCache` 借出来的，`nodes[start]` 与 `nodes[end]` 不是同一格，借用不冲突；如果借用检查报 `nodes` 同时可变借用，
先把 `ceilings` 里的前驱文本 `to_owned()`。）

- [ ] **Step 4：跑测试**

```bash
cargo test -p qingjian-core
```

预期：全部通过，包括三条新测试。

- [ ] **Step 5：clippy 与行数**

```bash
cargo clippy -p qingjian-core --all-targets -- -D warnings
wc -l crates/qingjian-core/src/sentence/viterbi/*.rs
```

预期：无警告；`mod.rs` 不超过 500 行左右（搬走测试后约 390 行）。

- [ ] **Step 6：提交**

```bash
git add crates/qingjian-core/src/sentence
git commit -m "feat(core): 整句格子按前文放进低频单读音词"
```

正文写清：首段 6 个不变，候补到 16 个，读音占比 ≥ 0.5 且某个前驱后比首段都可能才进；附 Task 5 的数字。

---

## Task 5：评测与计时

**Files:** 无代码改动。

- [ ] **Step 1：目标用例**

```bash
cargo build --release -p qingjian-cli
M="$HOME/Library/Input Methods/Sujian.app/Contents/Resources/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm"
QINGJIAN_API_KEY=x target/release/qingjian-cli --neural "$M" jishimu jishimude jishige 2>&1 | grep -E '^>|^\s+1\.'
QINGJIAN_API_KEY=x target/release/qingjian-cli jishimude 2>&1 | grep -E '^\s+1\.'
```

预期：接通变 几十亩 / 几十亩地 / 几十个；不接模型 `jishimude` 也是 几十亩地。

- [ ] **Step 2：整句评测、回放、计时，与 Task 0 对比**

```bash
QINGJIAN_API_KEY=x target/release/qingjian-cli --eval-text data/eval/sentences.tsv --eval-details "$TMPDIR/new.jsonl" 2>&1 | grep '^句子'
QINGJIAN_API_KEY=x target/release/qingjian-cli --neural "$M" --eval-text data/eval/sentences.tsv 2>&1 | grep '^句子'
QINGJIAN_API_KEY=x target/release/qingjian-cli --replay data/eval/input-log-2026-10-04.jsonl 2>&1 | tail -5
QINGJIAN_API_KEY=x target/release/qingjian-cli --typing woxiangzaishurudetongshijizhuyigecihuiranhoubatajiaruciku 2>&1 | tail -1
```

验收（spec「验收」一节）：冷启动与接通变的首选、前三、字准确率都不低于基线（原型是 33.1% / 38.9% / 77.0%）；
回放词首选、整句首选不低于基线；逐键平均增幅 ≤ 0.5 ms、最慢一键 < 10 ms。不达标就停下，把 `new.jsonl` 与 `base.jsonl` 逐句对账后回到 spec 调整，不硬调常数。

- [ ] **Step 3：逐句对账（翻好的与翻坏的各看一遍）**

```bash
python3 - "$TMPDIR/base.jsonl" "$TMPDIR/new.jsonl" <<'EOF'
import json, sys
b = [json.loads(l) for l in open(sys.argv[1])]
n = [json.loads(l) for l in open(sys.argv[2])]
for x, y in zip(b, n):
    ob, on = x["candidates"][0] == x["text"], y["candidates"][0] == x["text"]
    if ob != on:
        print("好" if on else "坏", x["text"], x["candidates"][0], "→", y["candidates"][0])
EOF
```

预期：翻坏的里没有成片的多音字冷门读音（和/huo、没/mo 这类）；有的话说明占比门槛没生效。

---

## Task 6：文档

**Files:**
- Modify: `docs/notes/crate-notes.md`（`qingjian-core` 整句一节）

- [ ] **Step 1：crate-notes 记常数与数字**

在整句候选那段（`SENTENCE_CANDIDATES` 附近）加一段：格子首段 `SPAN_CANDIDATES` = 6、候补到 `SPAN_POOL` = 16，候补要读音占比 ≥ `MIN_READING_SHARE` = 0.5
且某个前驱后静态概率高于首段最高值才进；`Dictionary::text_frequency` 懒建；Task 5 的评测数字与日期。用户行为没有按键 / 配置变化，不改 `docs/user/`。

- [ ] **Step 2：提交**

```bash
git add docs/notes/crate-notes.md
git commit -m "docs(core): 记整句格子候补词的常数与评测"
```

- [ ] **Step 3：真机验证（合并前）**

```bash
apps/macos/scripts/bundle.sh --install
```

在 TextEdit 里打 `jishimu`、`jishimude`、`huozhe`、`moshi`，确认首选分别是 几十亩、几十亩地、或者、模式；PR「怎么验证的」写明 macOS 版本与步骤。
