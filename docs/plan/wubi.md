# 五笔（形码）支持方案

2026-09-15 定。第一期做 **86 版**，平台范围 **Core + CLI + Windows**，配置收敛到 `[general] scheme` 并把配置迁移一起做。
本文记录技术判断与分期；实现要点落地后同步 `docs/notes/crate-notes.md`。

## 一、为什么五笔不能按双拼的方式接

现有代码里「输入方案」的抽象是**「把按键解码成拼音串」**，双拼与注音共用的是**拼音**这个底层，不是**方案**这个抽象：

```rust
// crates/qingjian-core/src/engine/setup.rs:79
pub(super) fn decode(&self, keys: &str) -> Option<EngineDecoded> {
    if self.zhuyin { Some(EngineDecoded::Zhuyin(crate::zhuyin::decode(keys))) }
    else { self.shuangpin.map(|s| EngineDecoded::Shuangpin(s.decode(keys))) }
}
```

解出来的 `EngineDecoded`（`engine/decoded.rs`）提供 `pinyin / tail / segmentation / marked / keys_for`，
随后原样进 `engine/query/mod.rs:79` 之后的词级查找、排序、整句。

五笔在这一层就不成立：码串不是拼音，切不出音节，没有读音，没有整句。拼音耦合在三个层次上：

| 层 | 耦合点 | 五笔的情况 |
|---|---|---|
| 解析 | `parser::segment`（`parser/mod.rs:54`）+ `SYLLABLES` 表（`parser/syllable.rs:7`） | 码串无需切分，按音节切分本身就是错的 |
| 词库 | `Dictionary` 的键是空格分隔的拼音串，查询接口全部收 `SyllablePattern`（`dictionary.rs:350-393`） | 需要按「码」索引，前缀匹配而非音节位置匹配 |
| 排序 | `ranking/scored.rs` 的 `full_last` / `coverage` / `abbreviated` / 模糊音 `penalty` | 码等长时这些字段全部退化，词频主导 |

所以「在 `shuangpin/` 旁边加个 `wubi/` 目录」不成立。

## 二、方案：两条平行管线

把方案抽象提升一层，**Engine 在 `query_inner` 顶部分派到两条平行管线**，而不是让五笔伪装成拼音：

```rust
// engine/query/mod.rs:62 query_inner 的开头
if let Some(code) = self.code {
    return Ok(self.query_code(keys, rest, start));  // 形码：码串 → 码表 → 候选
}
// 以下原有拼音管线不动
```

三条理由：

1. **现成的先例。** `query/mod.rs:103-129` 那个「第一个字母都切不动」的分支已经在返回
   `segmentations: Vec::new()` + `tail: keys` 的 `Query`，下游（`Engine::annotate`、输入日志、
   `note_displayed`、各壳渲染）本来就吃得下「无切分」的 `Query`。五笔复用这个形状，不新造结果类型。
2. **不该开的开关变成天然不存在**，而不是散落一地的 `if wubi`。沿用现有套路
   （`engine/correcting.rs:9` 对双拼直接返回 `None`、`engine/setup.rs:70` 的 `modes()` 返回 `ModeKeys::LETTERLESS`），
   在一个集中判断里关掉：整句 / Viterbi、神经重排、模糊音、拼写纠错与词图敲错边、中英混输、简拼、
   v / u / i 前缀快捷键（**`v` 与 `i` 在五笔里是字根键，必须让位**）。
3. **青简的产品特色全部自动保留。** `Learner`（词频 / 用户词 / 选择学习）、`Translator` + `Glossary` 释义、
   生词本 `VocabularyBook`、CEFR / JLPT 等级、`InputLog`、`UsageStats` 都按「上屏的词」工作，与输入方案无关。
   **五笔用户照样有候选旁的译文与生词统计**——这是「Core 才是青简」换来的好处，也是这个功能的卖点。

### 数据层：静态全量码表，不做运行时取码

新增 `CodeTable`（`crates/qingjian-dictionary/src/code_table.rs`，与 `Dictionary` 同属「文本 → 键」的查表；
只有一个文件、没有同词干的兄弟，按约定不另开目录）。词目与 `WordList` 一样是排好序的 `Vec`，
查询靠**前缀二分**——比现在按音节位置逐级收窄简单。`.qj` 容器加 `Kind::Code`（`crates/qingjian-format`，阶段 2）。

词组取码规则（二字词 2+2、三字词 1+1+1、四字及以上 1+1+1+1）**在生成数据时算好，运行时不算**：
规则连带简码与识别码很绕，而静态码表够小也够全；码表没有的词靠用户词学习补，不做运行时推导。

一级 / 二级 / 三级简码不专门实现——它们就是「前缀查询 + 词频排序」的自然结果（`g` → 一，`gg` → 该前缀下词频最高的字）。

### 用户词

现有自动造词（连着选出的两个词合成用户词）按「词的字数 = 音节数」判断，形码没有音节，自然不触发
（`try_auto_word` / `finish_buffer` 里 `chars != syllables.len()` 直接返回），不必另加开关。
「同一个码下选过的词靠前」由现有的 `Learner::record_choice` 负责，形码不需要新的用户词表。

### 阶段 1 落定的三件事（2026-09-15）

1. **候选是新的一种 `CandidateKind::Code`**，不是塞进 `Chinese`。它的 `syllables` 为空、上屏吃掉整段作用域
   （`commit` 里的 `whole_scope`）。这样「没有音节」是类型里的事实，而不是靠空串蒙混；整段造词也随之不触发。
2. **简码不用专门做**。一级 / 二级 / 三级简码就是「编码打全的词排在同前缀的长编码词前面」，
   由 `CodeTable::lookup` 的 `exact` 位给出，与拼音侧「音节数与输入一致者优先」是同一个排序位。
3. **码表第一期只读 TSV**，不进 `.qj` 容器（`Kind::Code` 留到阶段 2 与 `pack` 一起做）。
   段 1 的 CLI 入口是 `--wubi <码表.tsv>`。

## 三、数据

| 来源 | 许可 | 结论 |
|---|---|---|
| [rime-wubi86-jidian](https://github.com/sxjudya/rime-wubi86-jidian)（极点 86 码表） | Apache-2.0 | **采用**：宽松、词量最大 |
| [rime/rime-wubi](https://github.com/rime/rime-wubi) 官方 `wubi86.dict.yaml` | LGPL-3.0 | 备选：与本仓库 GPL-3.0-or-later 兼容，但要单独带 LICENSE |
| Gitee「大一统五笔」 | 未声明 | 不能用 |

**词频不用码表自带的权重**（那是码表顺序，不是语料词频）。用现有 `assets/lexicon/dict.tsv` 里
同一个词的语料词频交叉回填——换方案不会让同一个词的排序变奇怪，释义兜底与生词识别也对得上。

管线：`tools/dict-convert` 加 `wubi` 子命令读码表 → `assets/wubi/wubi86.tsv`（`词\t码\t词频`）
→ `pack code` → `code-wubi86.qj`。

`qingjian-dictionary/src/import/rime.rs` 已能把 Rime `.dict.yaml` 的 `词\t拼音\t权重` 转成青简 TSV，
五笔码表正好是这个格式（第二列是 `ggll` 这样的码），转换主体可以直接复用。

**许可与署名**（同步四处，见 `docs/design/landscape.md:55`）：`assets/wubi/LICENSE`、
`docs/design/landscape.md` 的数据源表与随包数据清单、README 的 License 一节、偏好设置「关于」页。

## 四、配置与壳

现状 `[general] shuangpin = ""` + `[general] zhuyin = false` 是**两个字段表达同一个维度**，
再加一个并列字段就是三个字段打架。**收敛成 `[general] scheme`**：

```toml
[general]
scheme = "pinyin"   # pinyin | xiaohe | ziranma | microsoft | sogou | zhuyin | wubi86
```

值就是现有 `ShuangpinScheme::key()`（`shuangpin/scheme/mod.rs`）与 `zhuyin` 的统一。
旧键读取时兼容、写入时迁移，落地时把 `docs/plan/todo.md` 里挂着的「配置文件版本迁移」一起做掉。

| 壳 | 落点 |
|---|---|
| Core | `engine/mod.rs` 的 `shuangpin` 与 `zhuyin` 收敛为一个字段；`engine/setup.rs:20-33` 的 setter 合并；阶段 1 已加的 `code: Option<CodeTable>` 改为随 `scheme` 装载（哪个码表由配置决定，Core 不需要 `CodeScheme` 枚举） |
| 配置 | `crates/qingjian-platform/src/config/general/mod.rs` |
| CLI | `apps/cli/src/args.rs:76` 加 `--wubi`；`apps/cli/src/main.rs:236-250` 应用；回放 `apps/cli/src/replay/mod.rs:95-116` 认新 scheme |
| Windows 设置 | `apps/windows/settings/src/panel/pages/general.rs:14-20` 的 `SHUANGPIN` 列表扩成「输入方案」；`panel/message.rs:12-13`、`panel/component.rs:41-44` |
| Windows Server | `server/src/main.rs:188-189` 启动应用；`dispatch/reload/mod.rs:107-108` 热加载；状态条 `dispatch/status/mod.rs:93-99` 用 `scheme.label()` |
| macOS | **本期不动**。顺带记一笔：`apps/macos` 至今没有接注音（全目录无 `set_zhuyin_mode` 引用），mac 接方案时要把注音一起补上 |

中英切换仍然是布尔，不变（架构约定「输入方案是配置项，不是模式」）。

## 五、分期与验收

| 阶段 | 内容 | 估计 |
|---|---|---|
| 1 ✅ | Core `CodeTable` + `query_code` + Engine 分派 + CLI `--wubi`（2026-09-15 完成，233 + 21 个测试全绿，CLI 端到端验过） | 3–5 天 |
| 2 | `dict-convert wubi` + 词频回填 + `pack code` + 许可署名 | 1–2 天 |
| | 　└ `dict-convert wubi` 与词频回填**已做**（2026-09-15，读 Rime `.dict.yaml`，词频按词面从青简词库回填，4 个用例）；剩下载码表、`Kind::Code` 打包与许可署名 | |
| 3 | `[general] scheme` 收敛 + 配置迁移 + Windows 设置页与状态条 | 2–3 天 |
| | 　└ **已做**（2026-09-16）：`Scheme` 枚举与 `[general] scheme`；旧键 `shuangpin` / `zhuyin` 读取时推断（不自动改写文件）；CLI、macOS 偏好设置与 Windows 设置页/Server/状态条都改成读写新键；Windows Server 按方案装载码表（用户目录优先、随包 `assets/wubi/` 兜底，找不到只警告并按拼音跑）。打包与 macOS 接入**也已做**（2026-09-16 续）：`bundle.sh` 拷进 `Resources/wubi/`、Windows 安装器拷进 `{app}\assets\wubi\`，两处署名（macOS / Windows 的「关于」页）都补了；macOS 的偏好设置那栏换成「输入方案」下拉（`Scheme::ALL`），`apply_config` 里一并设注音与码表——macOS 侧因此顺带把一直没接的**大千注音**也接上了。**还差**：macOS 与 Windows 的真机验证 | |
| | 　└ 壳真带上码表时，署名要同步：`bundle.sh` 是**按文件白名单**拷数据的（`assets/wubi/` 现在一个都没拷），偏好设置「关于」页的 `ATTRIBUTIONS` 也是照随包数据列的一份。现在两处都没加，因为 macOS 侧还没有输入方案，包里带它只是白占体积 | |
| 4 | 释义 / 生词 / 统计 / 日志对齐验证、输入日志 `scheme` 进回放、`docs/user/input/` 加页 | 1–2 天 |
| | 　└ **已做**（2026-09-16）：回放按每条日志的方案切换（`SchemeSwitcher`，形码要有 `--wubi` 的码表）；释义 / 生词 / 词汇记录 / 输入统计在形码下照常，有测试钉住（`code_commits_still_feed_translations_vocabulary_and_usage`）；`docs/user/` 的按键表、拼写纠错、英文模式、偏好设置、模糊音与输入方案各页都补了五笔 | |

合计约 1.5–2 周（单人，不含真机来回）。

验收沿用仓库规矩：改动先跑 `apps/cli`。`--typing` 看逐键延迟（形码每键应当比拼音更快，
没有整句与模糊音的开销）；`--replay` 看既有拼音日志有没有退化（应当完全不变）；
`--eval-text` 不适用于形码，跑一次确认没被误触发即可。

## 五之二、混输（2026-09-16 追加，原计划之外）

原计划里 `[general] scheme` 是**单选**的，选了五笔就完全不进拼音管线。实际用起来要的是
「双拼和五笔分开和组合」，所以拆成**两条独立的轴**：

```toml
scheme = "pinyin"    # 拼音侧：pinyin / xiaohe / ziranma / microsoft / sogou / zhuyin / none（关）
wubi   = "wubi86"    # 形码侧：空为关
```

三种用法：双拼 alone（`xiaohe` + 空）、五笔 alone（`none` + `wubi86`）、**混输**（两边都开）。
`Engine` 上对应两个开关 `set_code_table` / `set_phonetic`，`query_inner` 按它们分派到
`query_code` / `query_phonetic` / `query_mixed`。

### 排序：编码打全的形码词在前

2026-09-18 定：**编码打全的形码词 → 拼音候选 → 只命中编码前缀的形码词**。一律形码在前的话，
真实数据下 `kai` 的首选是编码 `kaik` 的 中共党员、`shi` 是 椒，拼音那半边没法用；打全的编码是精确的，
留在最前（`ga` → 开、`wo` → 伙）。混输下模式键同双拼换成大写（`V1+2`），四码以内不做拼写纠错。

另外两条规则也是拍板的：

- 拼音读不出来时（`ggll` 切不成音节）整个按形码走，两边都空才算错。
- 五笔码最长 4 位，**第 5 个字母起自然只剩拼音**，不用特判。

## 五之三、自定义方案与顶字（2026-10-20 追加）

形码侧从「只有 86 五笔」扩成**方案**：Core 的 `CodeScheme` = 码表 + 两条按键规则（满码唯一自动上屏、顶字）+ 日志键，
平台层 `code_schemes` 负责发现（内置 `wubi86` + 用户数据目录 `schemes/<id>/`，`scheme.toml` + 码表 TSV）。
配置收成 `[general] code = "<id>"`（`off` 为关），旧的 `wubi = "wubi86"` 读取时认、不再写出。

- 两条规则只在**只用形码**时生效：混输下满码之后的字母可能是在接着打拼音，自动上屏会替用户截断，顶字更不对。
- 顶字上屏的是壳里的**高亮**候选（用户可能挪过），所以 Core 只判断（`code_tops`），动作在壳；满码唯一自动上屏没有高亮可挪，Core 直接上屏（`commit_unique_code`）。
- 满码的编码一个候选都没有时不顶：顶了只会把字母原样上屏，不如停着让用户退格。
- 标点顶字与空码清除同样只在只用形码时：标点总是先上屏高亮候选（主流五笔的缺省，拼音那边仍看 `punct_commits`）；
  空码时空格与标点只清编码，不把字母原样打进正文，回车仍原样上屏（要打字母时有路可走）。
- 满码长度缺省取码表里**最常见**的编码长度（`CodeTable::full_code_len`），`scheme.toml` 的 `code_length` 可覆盖；取最长的话一条写错的 5 码（极点码表的 `TF卡 ttfhh`）就让顶字与满码上屏全部失效（2026-10-20 真机上踩到）；编码只认字母，别的键（`;` `,`）留给标点与翻页，含它们的行加载时跳过。
- 内置五笔的两条规则缺省开；用户目录放 `schemes/wubi86/` 可整个换掉内置那套（换码表或关规则）。
- 随包五笔本身也是一个方案目录（`assets/schemes/wubi86/`），与用户方案同一套找法，代码里不再有内置特例。
- 配置热加载不重读没变的码表（`SchemeSlot` 比方案与码表文件的修改时间、长度）。
- 二进制码表（`.qj` 加形码类型）先不做：每键查询本来就在内存里，二进制只省加载与内存，等实测加载时间再定；分享用的打包格式另议。
- Linux 壳还没接形码，与之前一样。

## 六、不做的事

- **运行时取码推导**（由单字码拼出词组码）：规则太绕，静态表够用。
- **形码下的整句转换**：五笔的整句没有意义，码长即词长。
- **临时拼音 / 反查（`z` 万能键）**：第一期不做，等有用户反馈再说。
- **98 版与新世纪版**：不随包；用户自己把码表放进 `schemes/` 就能用（见五之三）。
