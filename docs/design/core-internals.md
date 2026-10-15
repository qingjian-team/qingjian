# Core 与显示层拆解

[architecture.md](architecture.md) 写的是「为什么这么定」，这一页写「现在长什么样」：
`Engine` 的字段与线程归属、一次按键从键到候选走过的每一层、三条异步旁路、上屏时的写回、几处缓存，
以及显示层在三个平台上各画了什么、哪些地方还是各写一份。
读代码前先看这页，读完直接进 `crates/qingjian-core/src/engine/`。

2026-09-23 起稿，2026-10-14 对着 main（2b96d75，0.2.1 之后）核过一遍。实现改了要回来改这里；文中的常数以代码为准。

## 一、`Engine`：73 个字段，分六组

`Engine`（`engine/mod.rs`）是 Core 对外的唯一门面。字段按「谁改、多久改一次」分六组看：

| 组 | 字段 | 性质 |
|---|---|---|
| **数据** | `dictionary`、`extra_dictionaries`、`companion_words`（编进 Core 的符号 / emoji 名字，词库里没有）、`english`、`emoji`、`kaomoji`、`symbols`、`custom_phrases`、`code`（五笔码表）、`aux_codes`（辅码表）、`opencc` | 装配时给，热加载时整张换 |
| **注入口** | `translator`、`english_translator`、`learner`、`logger`、`predictor`、`language_model`、`sentence_scorer`、`rescorer`、`meter`、`vocabulary`、`gloss_filler` | trait 对象，全有 `No*` 空实现；`learner` / `logger` 外面各罩一层 `MutedLearner` / `MutedLogger`，私密输入时吞掉写入 |
| **配置** | `modes`、`fuzzy`、`shuangpin`、`shuangpin_raw_preedit`、`zhuyin`、`phonetic`、`full_width_punctuation`、`chinese_first`、`english_in_chinese`、`auto_space`、`emoji_limit` / `kaomoji_limit` / `symbol_limit`、`shift_letter_compose`、`interpolation`、`typo_costs`、`neural_weight` / `neural_margin` / `neural_context`、`aux_enabled` / `aux_show` / `aux_code_key` / `aux_keep_empty`、`traditional` | 壳用 `set_*`（`engine/setup.rs`）推进来；影响排序的顺手清缓存 |
| **会话状态** | `composition`、`english_mode`、`punctuation`、`history`、`chain`、`recent_commits`、`recording`、`retype_snapshot`、`passthrough_pending`、`page_turns`、`composition_started`、`application`、`committed_since_break`、`displayed` | 每键变；正好是 `EngineSession` 的 14 个字段，可以整组换出（见下） |
| **引擎级状态** | `private`、`aux_code`（码段）、`panel`（打开着的符号面板）、`rescoring_before`、`log_sequence`、`prediction_sequence`、`pending_predictions`（在飞的请求按序号记种类与作用域） | 每键变，但不随会话换 |
| **内部可变** | `span_cache`、`correction_cache`、`neural_cache`、`last_rescored`、`last_query`、`traditional_map` | `RefCell` / `Cell`：`query(&self)` 只读借用也要能往里写 |

`sentence_scorer` 是同步打分器（CLI 评测用），`rescorer` 是把它挪到后台线程的 `RescoreWorker`（壳用，`set_async_sentence_scorer` 装）。

### 三条对移植与并发都要紧的性质

- **注入的 trait 全部 `Send`，缺省实现全是空操作**（`Translator` / `Learner` / `Predictor` / `LanguageModel` /
  `SentenceScorer` / `InputLogger` / `UsageMeter` / `VocabularyTracker` / `GlossFiller`；辅码表 `AuxCodeLookup` 要多处共享，是 `Send + Sync`）。
  所以 CLI 和单元测试不需要真实词库、不联网、不落盘也能跑完整条查询路径——这是整个架构最值钱的一处设计。
- **`Engine` 是 `Send`、不是 `Sync`。** 内部可变那组让 `&Engine` 不能在两条线程上同时用，但整个 Engine 可以交给另一条线程。
  三个壳都是「一条线程独占，别的线程发消息过来」：

  | 壳 | 谁持有 Engine | 别的线程怎么够到它 |
  |---|---|---|
  | macOS | 主线程的 `thread_local!` 单例，`host::with` 取 | IMK 回调本来就在主线程；后台结果由主线程上的 `NSTimer` 轮询 |
  | Windows | Server 进程的 Router 线程 | 每条管道连接一条线程读消息，经 `mpsc` 交给 Router；画窗口的是另一条 UI 线程 |
  | Linux | Server 主线程上的 Router | 每条 socket 连接一条线程，经 `mpsc::sync_channel(128)` 交给 Router |

  任何新壳都照这个形状来：别想着给 Engine 加锁多线程共享，查询路径上的缓存都假定只有一个人在用。
- **会话状态可以整组换出。** `EngineSession`（`engine/session.rs`）就是会话状态那 14 个字段；`swap_session` 与它整组互换，
  顺手取消在飞的联想、清掉查询缓存，词库、学习数据与配置始终只有一份。Linux Server 靠它给每个输入上下文一份独立的会话状态
  （组句、中英模式、标点配对、上屏前文互不串）；macOS 与 Windows 目前不用它。`discard_input` 用在隐私边界：清掉组句，不写日志、不学习。
  **缺口**：辅码码段 `aux_code` 不在 `EngineSession` 里，切会话不跟着走、`discard_input` 也不清它。
  Linux Server 还没接辅码所以暂时碰不到，接之前先把它挪进会话。

## 二、一次按键的数据流

```text
壳: push(c) / backspace / move_cursor_* / push_aux_code …
      │
      ▼
Composition          拼音缓冲 + 光标；scope = 光标前那段（光标停中间时只按它出候选）
      │
      ▼
Engine::query()      engine/query/mod.rs，最热的一条路
      │
      ├─ query_inner：按顺序分派，先命中先走
      │     english_mode             → query_english     英文词表：精确词、前缀补全、拼错纠正
      │     表达式键开头（v）         → query_expression  算式结果、中文数字
      │     问字键开头（u / ?）       → query_question    本地没有候选，答案等云端；u4e00 这类码点直接给字
      │     is_raw（含 - . 这类）     → query_raw         英文直输段，唯一候选就是原文
      │     有码表、拼音关            → query_code        五笔：码表前缀查，没有切分 / 纠错 / 整句
      │     有码表、拼音开            → query_mixed       混输：形码「编码打全」的排最前，其余与拼音合并
      │     其余                      → query_phonetic   ↓
      │
      ├─ decode                双拼 / 注音先解成全拼（Decoded），之后与全拼同路
      ├─ split_english_tail    末尾是英文词（…xuehaorust）：与整段读成拼音比分（mixed_beats_plain），赢了才拆开
      ├─ parser::segment       trie 切音节；全拼 / 简拼混排 / 末尾残缺音节；整段切不动就取最长可切前缀；
      │                        同音节数时不带生僻音节（fiao / biang）的切法在前：biangeng 是 bian geng
      ├─ active_correction     拼音「不像话」（unlikely_pinyin）才试一处编辑的变体；按噪声信道，
      │                        纠后得分扣掉编辑代价仍高于原样才纠；结果按 scope 记进 correction_cache
      │
      ├─【词级】fuzzy.expand → lookup_all     每个位置扩成多种写法（z/zh、an/ang…），主词库、附加词库、用户词逐个查
      │         输入前缀 → lookup_exact_all  kaifazhe 也出 开发、开；同一次查询里同一个模式只查一遍（memo）
      │         ranking::rank                log P(词 | 上一个上屏词) + 封顶的选择次数加分，截到 500 条
      │
      ├─【辅码】aux_filter      码段非空时反向过滤：没有码的词藏掉，命中的按 完全匹配 > 码长 > 原序 重排
      │                        （下面几种附加候选都没有码，辅码筛词时一律不出）
      │
      ├─【整句】insert_sentence → sentence/
      │     词图：每格 lookup_exact，每格留前 6（含简拼的格子留 20），SpanCache 跨按键复用
      │           词频 0 的生僻字不进图（用户选过的除外），免得 keneng 按 ken eng 读出 肯鞥
      │           correction::typo 敲错边：每个完整音节的一处敲错变体带代价入图
      │     Viterbi + 束搜索（束宽 8）
      │           打分 = (1−μ)·静态 bigram + μ·个人 n-gram，μ = c/(c+8) 封顶 0.5
      │           个人侧先算二元（0.8 / 0.2 插值），上文见过再套绝对折扣三元（D = 0.75）
      │     rescore_paths：装了神经打分器时前 6 条路径按字级 Transformer 重排；
      │           分数从 neural_cache 取，缺分的攒着等壳送后台，有一条缺分就不动顺序；
      │           任一条带打分器字表外的字也不动（那个字只能当 <unk> 算，整条会被压到底）
      │
      ├─ insert_english        与整句谁先进看 chinese_first
      ├─ insert_shortcuts / insert_emoji   emoji、颜文字、符号三张表按各自名额配在对应的词后面
      │
      ▼
query() 收尾          aux_segment；insert_custom_phrases（辅码态不出）；写 last_query 摘要给输入日志；
                     繁体开着时 Chinese / Sentence / Cloud 候选过 OpenCC，繁→简记进 traditional_map
      │
      ▼
Query{candidates, segmentations, tail, correction, aux, …}
      │
      ▼
Engine::annotate()   Translator 查本地释义 → Translation{senses}
      │              VocabularyTracker 标 Sense::fresh（看到的轮次不到 FRESH_UNTIL = 3 即生词）
      ▼
壳: 画一帧
```

形码那条路不经过切分、纠错与整句（编码不需要切，没有简拼与模糊音，一处编辑的纠错也不适用），
但译词标注、生词记录、输入日志、选择学习与个人 n-gram 都按上屏的词工作，与拼音共用；取舍见 [plan/wubi.md](../plan/wubi.md)。
辅码的触发、过滤与学习语义见 [aux-code.md](aux-code.md)。
各层常数为什么是这些值，见 [architecture.md](architecture.md)「Core 的关键技术决定」与 [notes/constant-sweep.md](../notes/constant-sweep.md)。

## 三、三条异步旁路

三条都遵守同一条铁律：**不阻塞候选生成**——结果没到就先按本地的出，到了壳再重画一次。
云端词只进 `CandidateLayout` 留的固定槽位，不挪本地候选；神经分只在整句那几条路径之间调顺序。
Linux Server 只接了神经重排（#214）：Server 里同样停键 80 ms 送去、每 20 ms 收、最多等 2 s；
Linux 协议是一问一答、Server 不推送，重排结果靠 fcitx5 插件组句期间每 80 ms 发一次 `Poll` 取回。云联想与释义兜底没接。

| 旁路 | trait | 后台线程 | 壳怎么取结果 | 丢不丢得起 |
|---|---|---|---|---|
| 云联想 / 问字 | `Predictor` | `qingjian-predict` 自开，防抖 300 ms、缓存 64、超时 5 s | `poll_prediction()` | **丢得起**：组句联想与整句译文各只认最新一条，可同时在飞（`pending_predictions` 按序号记着），清空输入时连还没发的一起撤掉 |
| 神经重排 | `SentenceScorer` | `rescoring::RescoreWorker` | 停键 80 ms 后 `request_rescoring()`，每 20 ms `poll_rescoring()`，最多等 2 s | 丢得起：没到的分数就按静态模型排 |
| 释义兜底 | `GlossFiller` | `qingjian-predict` 独立线程，攒 1.5 s 或 8 个词 | `poll_glosses()` | **丢不起**：每个词都要问到，慢点没关系 |

同样是「异步调云」，因为丢得起和丢不起，`Predictor` 与 `GlossFiller` 是两个 trait、两条线程、两套策略，
没有合并成一个通用的异步任务队列。这条分界是有意的，见 [architecture.md](architecture.md)「翻译数据本地化」。

私密输入时两条云端旁路都不发（联想请求前查 `private`，释义兜底在上屏时跳过），`set_private(true)` 还把 `prediction_sequence` 加一，
在飞的联想回来也作废；神经重排在本地照跑，但光标前文清掉、不带进模型。

## 四、上屏时的写回

`Engine::commit()` → `commit_with()`（`engine/commit/mod.rs`），按代码里的顺序：

```text
commit(candidate)
 ├─ 关符号面板；开了自动空格（set_auto_space）时先看上文要不要补一个前导空格
 ├─ forget_span_cache          先清格子缓存：下面每一路都可能改排序
 ├─ Learner，按候选种类分路
 │    Chinese / Code            record（词频）+ record_choice（这段输入串下选了它）
 │    Cloud                     同上；词库里没有的先 learn_word 记成用户词
 │    English                   record + learn_english（进个人英文词表）
 │    Sentence                  不记词频，路径上的词在下面逐条记转移
 │    Emoji / Kaomoji / Symbol  按对应词的音节吃拼音，不学习
 │    Generated                 模型直接生成的整句，没有音节对齐，吃掉整段作用域，不学习
 │    Shortcut / Custom         不学习
 ├─ apply_retraction           删光重打换了个词：上一次记的词频、选择、转移、敲错全部退回
 ├─ record_typo                接受的 (敲的, 要的) 音节对 ──► user-typos.tsv
 ├─ log_commit                 input-log.jsonl 一行（键按原样记，辅码态把触发键与码段接在后面）
 ├─ meter_commit               汉字 / 中文词 / 英文词计数 ──► usage.tsv
 ├─ vocabulary.record_commit   带译词的中文候选记「上屏过」──► user-vocab.tsv
 ├─ gloss_filler               词库有、释义表没有的词交给后台问云端，回来写进个人释义表
 ├─ record_word                个人 n-gram：用户点选的词记 2 份，整句路径顺带的记 1 份
 │    └─ try_auto_word         连选两词、合起来 ≤ 4 字且哪里都没有：同一段拼音里选出 2 次、分两段打 3 次就成用户词
 └─ finish_buffer              一段拼音分几次才选完：整段合起来记一次选择，够次数也造词
```

两层包装罩在外面：

- **私密输入**：`set_private(true)` 让 `MutedLearner` / `MutedLogger` 生效——**写吞掉、读照常、排序不变**；
  词汇记录与释义兜底在 `commit_with` 里跳过，输入统计照记（只有数字、没有文本）。
  判定在壳：Windows 看 TSF 的输入范围，Linux 看前端报来的能力位（密码 / 敏感）；
  macOS 在 Secure Input 下不组句，所以壳不调 `set_private`，只额外挡掉云联想、神经重排的前文与翻译快捷键。
- **退格撤销**：`recent_commits` 留最近 4 次上屏，壳把组句外的退格用 `note_backspace()` 告诉 Engine，
  删光再重打同一段拼音换了个词，就走上面的 `apply_retraction`。**目前只有 macOS 调了 `note_backspace`**，
  Windows 与 Linux 的 Server 都没调，这两个平台上撤销不会发生。

## 五、缓存与内部可变状态

性能主要来源于前三处，动它们要用 `qingjian-cli --typing` 的 release 构建重量每键耗时：

| 名字 | 键 → 值 | 什么时候清 |
|---|---|---|
| `span_cache`（`sentence::SpanCache`） | 格子模式（含模糊写法）→ 排好截好的格子候选 | 上屏、删词、换学习数据 / 词库 / 模糊音这类配置、换会话；超过 8192 格整个清 |
| `correction_cache` | scope → 纠错结果（含「不用纠」） | scope 变了自然重算；`take_raw` 原样上屏（记成不再纠）、删词、改配置时清 |
| 词库前缀记忆化 | 模式 → 命中 | 单次查询内的局部 `HashMap` |
| `neural_cache`（`rescoring::NeuralCache`） | 前文 + 整句文本 → 神经分；外加「等着送后台」的文本 | 前文变了整张清 |
| `last_rescored` | 这次查询有没有动用神经分 | 每次查询开头复位 |
| `last_query`（`QuerySnapshot`） | 上一次查询的摘要：scope、切分、前 32 个候选 | 每次查询覆盖；给输入日志接上「看到了什么」与「选了什么」 |
| `traditional_map` | 繁体候选文本 → 原简体 | 组句清空 / 原样上屏 / `discard_input` 时清；上屏与删词靠它把学习记回简体 |
| `lm` 的 CSR bigram | — | 只读，随 `.qj` mmap |

历次优化的起因、定位方法与前后数字在 [notes/performance.md](../notes/performance.md)。

## 六、显示层

### Core 交出什么

壳拿到的是**已经排好序、已经带好 annotation** 的数据，**一行排序逻辑都不在壳里**：

```text
Query{ candidates: CandidateList, segmentations, tail, correction, aux: Option<AuxSegment>, … }
  .marked_segments() -> Vec<MarkedSegment{text, kind}>     kind: Typed | Rest | Corrected | AuxCode
CandidateList{ items: Vec<Candidate> }
  Candidate{ text, kind, syllables, reading, translation: Option<Translation>, aux_code }
    kind: Chinese | Code | English | Cloud | Shortcut | Custom(i) | Emoji | Kaomoji | Symbol | Sentence | Generated
    Translation{ senses } ─► Sense{ part_of_speech, text, reading, fresh }
      .furigana() -> Vec<FuriganaSegment{text, reading}>  日文汉字段注平假名

CandidateLayout / Cell            分页排布：本地候选 + 云端固定槽位（[predict] slots）
Engine::raw_preedit() -> RawPreedit{text, cursor_bytes}   不查询、不学习地读原样键串（失焦原样上屏这类场合）
```

连「云端词补进第一页末尾几格、前面的本地候选一格不挪」这种排布规则都在 Core 的 `CandidateLayout` 里。
Windows 与 Linux 的 Engine 在 Server 进程里，与画面之间隔着一层协议。Windows 是 `qingjian_platform::protocol` 的 `Frame`
（拼音行分段、光标、当前页候选、高亮、页码、排布与主题），释义晚到的用后续的 `Update` 补；
Linux 在 `apps/linux/server/src/protocol/` 另有一套 JSON 消息，候选连译文一起发给 fcitx5 插件。
两边开会话时带的是**同一个** `PROTOCOL_VERSION`，Linux Server 版本不等就断开：Windows 那边加消息升版本，Linux 插件也得跟着变
（插件在 C++ 里写死这个数，`apps/linux/server/tests/protocol.rs` 读插件源码核对，忘了改 `cargo test` 就过不了）。

### 候选窗只有四个原语

若干行 (序号, 候选词, 译文) / 一行高亮 / 跟随光标定位 / 异步补画译文。
技术选型与被否掉的方案（WebView、egui / iced / GPUI）见 [candidate-ui.md](candidate-ui.md)；
自绘渲染器 `crates/qingjian-render` 的设计与验收见 [rendering.md](rendering.md)。

### 三个平台各画了什么

| | macOS | Windows | Linux（fcitx5） |
|---|---|---|---|
| 在哪画 | 输入法进程内，主线程 | **Server 进程**的一条 UI 线程（HWND 只在该线程碰，Router 经通道 + `PostThreadMessageW` 交过去） | fcitx5 进程内的 C++ 插件；Server 只出数据 |
| 谁画 | `qingjian-render` 出位图，`drawRect:` 里贴；`[general] renderer = "system"` 退回旧的 AppKit 自绘 | `qingjian-render` 出位图，`UpdateLayeredWindow` 贴；`system` 退回 GDI | fcitx5 自己的输入面板（`setCandidateList` / `setPreedit`），外观跟 fcitx5 主题走 |
| 窗口 | 非激活 `NSPanel`，level 101（与系统候选框同级） | 分层窗，圆角与阴影在 `ui/layered/` 合成 | fcitx5 管 |
| 置顶 | `CanJoinAllSpaces` + `FullScreenAuxiliary` | exe 的 `uiAccess` manifest + 代码签名 + 装 Program Files | fcitx5 管 |
| 定位 | IMK `attributesForCharacterIndex:lineHeightRectangle:` | DLL 量 `GetTextExt` ──► `PositionCandidates{rect}` | fcitx5 管 |
| 译词 | 全部义项、词性、假名注音，生词用强调色 | 同 macOS，另显示辅码 `[码]` | 只显示第一条义项，生词后缀「 · 生」；实际显示了哪几条回报给 Server（`DisplayReporting`），Server 只对这些调 `note_displayed` |
| 重排结果回到面板 | 主线程 `NSTimer` 轮询 Engine（20 ms 一次，最多 2 s） | DLL 组句期间每 80 ms 发 `Poll` 拉异步结果（重排、云端候选都走它），Server 回新帧 | 插件组句期间每 80 ms 发 `Poll`；帧没变时 Server 沿用展示版本号，插件看版本号变新才重画 |
| 踩过的坑 | macOS 26 把面板绑死在创建时已有的 Space，全屏应用里看不见；每次 `orderFront` 后查 `isOnActiveSpace`，不在就把内容视图搬到新建面板 | 普通置顶窗被微软商店 / 任务栏搜索的高 z-band 盖住；整个渲染层从 DLL 搬进 Server 进程 | 面板更新是异步的，点选、翻页都带 `revision` / 显示标识回来，对不上的丢掉 |

### 还是各写一份的地方

这几处重复都已经开始分叉，是下一步收口的对象：

1. **候选 → 行。** macOS `apps/macos/src/candidates/row.rs`（自己的 `Row` / `Tone`，再经 `bitmap/convert.rs` 转成渲染器的类型）、
   Windows `apps/windows/server/src/ui/candidates/row.rs`（直接产渲染器的 `Row`）、Linux `apps/linux/fcitx5/src/qingjian.cpp`（C++ 拼 comment）。
   现状：Windows 在行里显示辅码而 macOS 没有（macOS 也还没接辅码），macOS 把自定义短语截成 60 字预览而 Windows 不截，
   云端标记 macOS 在 `host/presenting` 里另设、Windows 在转换时就设，Linux 只取第一条义项。
   收口方向：`from_candidate` 写一份放进共享 crate，macOS 删掉自己的 `Frame` / `Row` 直接用 `qingjian_render` 的（`convert.rs` 文件头已经这么写）。
2. **按键分流。** macOS `apps/macos/src/imk/controller/{text,command}.rs`、Windows `apps/windows/server/src/dispatch/key/`、
   Linux `apps/linux/server/src/dispatch/key/`。后两份文件结构一模一样，Linux 那份是从 Windows 复制的，已经分叉：
   Windows 有辅码的四处分支，Linux 有 Delete 向后删。两个 Server 吃的都是 `qingjian_platform::protocol::KeyEvent`，
   最现实的一步是先把这两份合成一份放进共享 crate，macOS 的再看。
3. **Engine 装配与设置下发。** 三个壳各自决定装哪些 Translator / Learner、调哪些 `set_*`，已经不一样了：

   | | macOS | Windows | Linux Server |
   |---|---|---|---|
   | 神经重排（`set_async_sentence_scorer`） | ✓ | ✓ | ✓ |
   | 云联想 / 释义兜底 | ✓ | ✓ | — |
   | 五笔（`set_code_table`） | ✓ | ✓ | — |
   | 辅码（`set_aux_codes`） | — | ✓ | — |
   | 繁体（`set_traditional_mode`） | ✓ | ✓ | — |
   | 退格撤销（`note_backspace`） | ✓ | — | — |
   | 符号面板（`open_symbol_panel`） | ✓ | ✓ | — |
   | 中西文自动空格（`set_auto_space`） | ✓ | — | — |
   | 多会话（`swap_session`） | — | — | ✓ |

   这里只是记下现状，不是说每个壳都该补齐；但新加一项 Core 能力时，要在这张表里写清楚哪些壳接了。
