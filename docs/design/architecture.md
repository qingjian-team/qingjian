# 架构

## 总体结构

```text
                    Qingjian Core
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
   macOS Adapter    Windows Adapter    Linux Adapter
       IMK               TSF          IBus / Fcitx
        │                │                │
        ▼                ▼                ▼
  Candidate UI      Candidate UI      Candidate UI
```

词库、拼音解析、候选生成、排序、用户词频学习和翻译能力全部属于 Core。
平台层只做两件事：把系统输入事件翻译成 Core 的输入，把 Core 返回的候选画到候选窗口。

判断标准：把 IMK 换成 TSF，不应该需要改 Core 的任何一行。

## 架构约束

这些是核心设计决定，不要违反。

1. **Core 平台无关。** `qingjian-core` 及其兄弟 crate 不允许依赖任何平台 API。
   平台层里不允许出现排序逻辑、词库访问或翻译调用。
2. **一个候选词只显示一种辅助语言。** 用户配置 Primary Language + 单个 Learning Language。
   不要设计成 `translations: Vec<Translation>` 或 `HashMap<Lang, String>` 这类多语言并列的数据结构，
   那会在 API 层面把「一次只学一种语言」这条产品原则给破坏掉。
   翻译是候选词的 annotation（可选、单条），不是并列的第二套候选系统。
3. **输入优先于学习。** 任何为学习功能增加的延迟、弹窗、UI 干扰都是设计错误。
   翻译查询不能阻塞候选生成，Core 必须能在翻译尚未就绪时先返回候选。

## Workspace 结构

```text
qingjian/
├── crates/
│   ├── qingjian-core/          # composition / parser / correction / candidate / ranking / sentence / engine …（下面单列）
│   ├── qingjian-dictionary/    # 词库加载与查询
│   ├── qingjian-translate/     # 候选翻译 annotation
│   ├── qingjian-learning/      # 用户词频、用户词、个人英文词、个人 n-gram、个人敲错表（user.tsv / user-words.tsv / user-english.tsv / user-ngram.tsv / user-typos.tsv）、输入日志（input-log.jsonl）、输入统计（usage.tsv）、词汇记录（user-vocab.tsv）
│   ├── qingjian-predict/       # 云联想：Predictor 的网络实现（OpenAI 兼容接口）
│   ├── qingjian-lm/            # 整句转换的 bigram 语言模型：LanguageModel 的实现
│   ├── qingjian-neural/        # 字级 Transformer 的本地推理（candle）：SentenceScorer 的实现，给整句前几条路径重打分
│   ├── qingjian-format/        # .qj 数据容器：mmap 打开、零拷贝视图、写入器、可落盘的哈希索引（dictionary / lm 依赖它）
│   └── qingjian-platform/      # 平台层共用的部分：配置文件、协议类型
│
├── apps/
│   ├── cli/                    # 测试工具：查询、逐键计时、输入日志回放评测、整句评测
│   ├── macos/                  # IMK 输入法壳（app / host / imk / candidates / menubar / preferences）
│   ├── windows/                # Server 进程（IPC 分派 + Engine + 命名管道）
│   ├── windows-tsf/            # TSF 文本服务 DLL（cdylib）：COM 链路 + 连 Server 的管道客户端
│   └── linux/                  # 规划
│
├── tools/
│   ├── dict-convert/           # 产品数据生成：lexicon / bigram / mine / english / emoji / pack
│   ├── gloss-gen/              # LLM 批量生成释义表与多音字标注
│   └── corpus/                 # 语料预处理脚本（uv）
│
├── assets/                     # 随仓库的产品数据源：词库源、释义表、emoji 表、词汇等级表（levels/，CEFR-J / Octanove / JLPT）、图标、样例
├── data/                       # gitignore：语料、Unihan、生成物 data/generated/
├── docs/
└── README.md
```

`qingjian-core` 内部模块：

```text
qingjian-core
├── composition     # 输入状态机：拼音缓冲、光标、上屏
├── parser          # 拼音切分（全拼 / 简拼 / 双拼 / 模糊音）
├── candidate       # 候选数据模型（Candidate / Translation / Sense / PartOfSpeech / Language）；layout 是分页排布（本地候选 + 云端固定槽位），各平台壳共用
├── ranking         # 候选排序
├── shortcut        # 快捷候选：日期 / 时间 / 星期、v 表达式模式（四则运算、中文数字），不查词库
├── english         # 英文模式候选：词表精确词 / 前缀补全 / 一处编辑纠正（edit.rs），大小写跟着敲的走
├── sentence        # 离线整句转换：词图 + bigram Viterbi + 束搜索，LanguageModel trait（qingjian-lm 实现，缺省退化为一元），UserNgram 个人 n-gram（二元 + 三元），Context 上文（前两个词），
│               #   SentenceScorer trait（qingjian-neural 实现）：convert_paths 出前 K 条路径，Engine（engine/rescoring）按 路径分 + λ·(神经分 − 静态分) 重排，异步时后台线程打分、壳停顿后取
├── emoji           # emoji 候选：EmojiTable（词 → emoji，Unicode CLDR 中文 annotations）
├── fuzzy           # 模糊音：FuzzyRules（配置 [fuzzy]）把每个音节扩展成多种写法，Expanded 借出给词库多写法查询
├── shuangpin       # 双拼：Scheme 四套方案的键位表，decode 把敲的键解成全拼（音节间带 '），Decoded 把上屏消耗换算回键数；切分之后全部复用全拼
│                   # engine::ModeKeys（配置 [shortcut]）：表达式 / 问字前缀键，只能是 v / u / i；问字键后跟十六进制出码点字符
├── engine          # 对外门面：Engine，以及 Translator / Learner trait 与空实现
└── storage         # 小文件落盘原语：write_atomic（临时文件 + fsync + 改名）、read_text_lossy；学习 crate 与配置都用它
```

词库内存布局（`qingjian-dictionary`）：词文本与拼音键各放一个连续 arena，词目只存 `u32` 偏移 + 词频，
键按字节序排好（当年 89 万条的测试词库约 150 MB RSS，比 `String` + `Vec<String>` 的朴素布局省三分之二；现在产品词库 8.7 万条，
且 `.qj` 是 mmap 直接映射，见「数据文件」）。
查询接口是 `lookup_pattern(&[SyllablePattern])`（命中音节数 ≥ 模式长度）与 `lookup_exact`（正好等长），每个位置可以是
完整音节或前缀 / 声母。实现是逐级前缀收窄：「以某段前缀开头的键」在排好序的索引里总是连续区间，完整音节直接二分到
`前缀 + 音节 + 空格`，简拼位置按区间里实际出现的音节跳块（每块看第一条键就能二分出块尾），区间小于 48 条就改线性比对；
代价与匹配到的音节组合数成正比，与首音节下有多少键无关。每个位置可以给多种写法（`lookup_pattern_alt` / `lookup_exact_alt`，
模糊音用），同一位置的写法在每级逐个走、代价相加不相乘；调用方保证同一位置的写法互不覆盖。Engine 侧同一次查询里相同的前缀模式只查一遍，
排序键预计算、远超 500 条时先 `select_nth` 再排，候选最多给壳 500 条。

拼音切分（`parser`）：按位置做动态规划，每个位置只保留最优 8 种前缀切分，token 可以是完整音节、
声母（简拼）或末尾未打完的前缀。排序键：音节少 > 不完整音节少 > 前面的音节长。

中英混输（`Engine::insert_english`）：整段输入（不含 `'`）在英文词表里就加一个 `CandidateKind::English` 候选，
上屏吃掉整段输入。排第一的条件：切不动（有未切分尾部），或最优切分除末尾外还有不完整音节
（`hello` → `he l l o`）；否则排第二（`china` 是干净的 `chi na`）。词表 `WordList` 在 dictionary crate。

代码组织约定（2026-09-03 起）：一个 struct / enum / trait 及其 impl 单独一个文件，
模块文件只做 `mod` 声明、re-export 和自由函数；结构体字段逐条 `///` 注释并用空行分隔；
`thiserror` 的 `#[error]` 文案用英文，日志与 UI 文案用中文。

`Engine` 的会话 API：`set_input / push / backspace` 喂拼音，`query()` 返回不带译文的
`Query { segmentations, candidates, timings }`，`annotate(&mut CandidateList)` 补译文，
`commit(&Candidate)` 上屏并喂给 Learner（词频、词转移、自动造词）。`Learner::flush()` 由壳在退出 / 停用时调用，
失败只记日志不返回错误；壳停用时还调 `break_chain()`，之后上屏的词按句首记；
`Engine::flush_learning()` 把学习数据与输入日志一起落盘且不作废格子缓存，壳激活期间也定时调它。

### 崩溃不丢：原子写、损坏容忍、panic 隔离

输入法进程随时会被 launchd 杀掉或自己崩掉，用户攒的学习数据和正在打的字都不能因此没了：

- **原子写**（`qingjian_core::storage::write_atomic`）：学习 crate 的六张 TSV、输入统计 `usage.tsv`、词汇记录 `user-vocab.tsv`、`config.toml`、`.env` 都先写同目录的临时文件，
  flush + fsync 后改名覆盖；任何时刻磁盘上要么是旧文件要么是新文件。输入日志 `input-log.jsonl` 是追加写不走这条路，
  崩溃最多留半行，回放工具按行跳过坏行并计数。
- **损坏容忍**：学习数据各文件按行解析，格式不对的行记一条警告跳过（下次落盘就清掉了），编码坏掉的字节按替换字符读进来；
  只有权限、坏盘这类真正的 io 错误才算读失败，这时壳退回只在内存里学习（不带路径，不会拿空表覆盖用户的文件），输入法照常启动。
- **panic 隔离**：`define_class!` 生成的 IMK 回调是 ObjC 运行时直接调的，panic 穿出去进程就没了。壳在按键处理、
  `commitComposition`、`activateServer` / `deactivateServer`、定时器这些边界都用 `catch_unwind` 拦住（`imk::catch_panic`），
  拦下后把缓冲区里的字母原样交给应用、清引擎状态、收窗口（`imk::recover_from_panic`），按键交还给应用；
  `main.rs` 装的 panic hook 只记位置与 backtrace 进日志。
- **回调重入**：`Host` 是主线程 `RefCell` 单例（`host::with`）。闭包里凡是碰应用那边的东西（`surrounding_text` 读上下文、
  `caret_rect` 取光标、`insert_text`）都要等应用回话，IMK 在等的时候会跑一轮 run loop，`deactivateServer:` 之类的回调就可能在
  借用期间进来（2026-09-07 真发生过：`refresh → request_prediction → surrounding_text` 期间收到 deactivate，`borrow_mut` panic，
  善后再 panic，进程退出重启）。规矩是 IPC 一律放在 `with` 之外、分两次借；`with` 本身用 `try_borrow_mut`，借不到记 warn 返回 `None`，
  再有漏网的重入也只是跳过一次调用，进程不死。
- **有界丢失**：学习数据除了停用时保存，激活期间借每秒看配置文件的定时器每 60 秒 flush 一次（没有新数据时是空操作），
  被杀最多丢一分钟的学习。

实际结构会随开发调整，调整后同步更新这里。

## crate 依赖方向

```text
qingjian-dictionary        （纯数据加载与查询，不依赖任何兄弟 crate）
        ▲
qingjian-core              （定义 Translator / Learner / Predictor trait，依赖 dictionary）
        ▲           ▲            ▲
qingjian-translate  qingjian-learning  qingjian-predict  qingjian-lm  qingjian-neural   （实现 core 的 trait，依赖 core；learning 另依赖 translate 的 LevelTable 做词汇按级汇总）
        ▲           ▲            ▲
qingjian-platform          （配置文件 Config：general / shortcut / fuzzy / predict 分节，toml_edit 原地改键保留注释；协议类型，可序列化；依赖 core、predict）
        ▲
apps/*                     （组装：Engine::new(dict).with_translator(..).with_learner(..).with_predictor(..)）
```

`apps/cli` 是 Phase 1 的测试壳：`cargo run -p qingjian-cli -- kaifa` 直接查询，
不带参数进入交互模式（拼音查询、序号上屏、`:q` 退出），`--user-dict` 指定用户词频文件，
`--language en|ja` 或环境变量 `QINGJIAN_LEARNING_LANGUAGE` 选学习语言。
`--typing` 是性能测试模式：把输入当一键一键敲进去，每个前缀查一次并标注译文，一行一键打印各阶段耗时
（这是输入法每键的真实工作量，联想在后台线程不算），启动日志里带各数据文件的加载耗时。
性能改动要用 release 构建跑它看数字，目标每键 10 ms 以内。

- Core 只依赖 dictionary，不依赖 translate 和 learning。翻译与学习通过 trait 注入（`Translator` / `Learner` / `InputLogger` / `UsageMeter` / `VocabularyTracker`，
  缺省实现都是空操作），这样 Core 的单元测试和 CLI 工具不需要真实词典也能跑。
- `qingjian-platform` 里的类型必须可序列化（serde）：macOS 和 Linux 上 Core 与壳同进程，
  Windows 上 Core 在独立 Server 进程，同一套协议类型两边都用。
- `storage` 只放 Core 自己的持久化原语，用户词频的数据模型归 `qingjian-learning`。

## 翻译的异步模型

Core 的会话 API 分两步返回：`update(input) -> CandidateList` 立即返回不带译文的候选；
译文由 translate 在后台查表，通过 `poll_annotations()` 或回调补上。
本地查表通常在一次事件循环内就绪，但接口上必须允许「候选先到、译文后到」，
平台层收到 annotation 更新后只重绘对应行。

## Core 的关键技术决定

### 多词库

Engine 查词的词库是一个列表：主词库（随包 `dict.qj`）、附加词库（`Engine::set_extra_dictionaries`）、用户词（Learner 持有）。
三者一起进词级查询和整句词图；附加词库不带语言模型，它的词在路径上按词频兜底打分（`sentence::fallback_log_prob`），
所以领域词库、导入的第三方词库只影响「有没有这个词」和它的词频，不改变语言模型的尺度。
**领域词拆开**（2026-09-06）：`dict-convert lexicon` 把 THUOCL 领域词按来源文件拆成 11 本 `dicts/<领域>.qj`（法律 / 医学 / 地名 / 成语 /
诗词名句 / IT / 财经 / 饮食 / 动物 / 汽车 / 历史人物，各带 META），只有语料里出现 ≥ 50 次的领域词（`--domain-keep-min`）留在基础词库
（它们其实是通用词：医疗器械、侵权行为）；基础词库从 22 万条降到 8.7 万条、`dict.qj` 10 MB → 3 MB，领域词库合计 13 万条 7 MB。
分词统计语料时仍把 `dicts/*.tsv` 一起当词表，词表与拆分前一致，语言模型不用重跑。
壳负责装配，附加词库有两处：随包的领域词库在 `.app` 的 `Resources/dicts/`，缺省关闭，配置 `[dictionaries] domains` 列出打开的
（缺省只有 `idioms`，偏好设置「词库」页可勾选、不能移除）；用户自己导入的放用户目录 `dicts/`（macOS 是 `~/Library/Application Support/Qingjian/dicts/`），
目录里的 `.qj` / TSV 文件全部加载，配置 `[dictionaries] disabled` 列出要关掉的文件名；导入 = `qingjian_dictionary::import`
把青简 TSV / Rime `.dict.yaml` / `.qj` 转成 `.qj` 放进去（Rime 的 YAML 头只取 `name:`，权重非整数当 1），
移除 = 文件挪到 `dicts/removed/`，开关 = 改配置，三个动作之后 `Host::reload_dictionaries` 重新装配。这也是第三方词库带着自己许可证单独分发的落点：
`.qj` 的 `META` 里有名称与许可证，偏好设置里直接显示。

### 数据文件：`.qj` 容器

词库、语言模型这类常驻数据用自己的二进制容器 `.qj`（`crates/qingjian-format`），原则是**内存布局就是文件布局**：
从 TSV 解析出来的几段连续数组（词库的词文本 arena、拼音键 arena、键索引、词目；语言模型的词 arena、词表、哈希索引、
CSR 偏移与后继）原样落盘，打开时 mmap 整个文件、校验一遍头与分节边界，不反序列化。启动从 0.9 s 降到 50 ms。

- 文件 = 32 字节头（魔数 `QINGJIAN`、格式版本、数据种类 `Kind`、分节数）+ 分节表（4 字节标签 + 偏移 + 长度，正文 8 字节对齐）
  + 各分节。第一节固定是 `META`：TOML 的 `Metadata`（名称、许可证 SPDX、署名、来源、版本、条数、生成者），
  偏好设置里的词库列表直接显示它，第三方词库各带各的许可证靠的就是这一节。
- 数值小端、原生对齐，crate 在大端机器上拒绝编译。字符串分节打开时校验一次 UTF-8，之后 `Text::deref` 走 unchecked
  （曾经每次 deref 都重新校验 30 MB，CLI 直接卡死）。定长结构体用 `zerocopy` 派生，`#[repr(C)]` 且手工排字段消灭填充
  （`KeyIndex` 16 字节、`Slot` 12 字节、`WordEntry` 12 字节、`Successor` 8 字节）。
- 两种视图：`Table<T>`（`Owned(Vec<T>)` / `Mapped`，`Deref<Target = [T]>`）与 `Text`（`Owned(String)` / `Mapped`，
  `Deref<Target = str>`）。解析路径与映射路径产出同一种结构，查询代码不区分。
- 文件里的哈希索引（`qingjian_format::hash`）：开放寻址、槽里放条目编号、键留在 arena；哈希函数必须跨进程、跨版本稳定
  （写文件的进程和读文件的进程算出来要一样），用 FNV-1a 64 加 fmix64 终混（FNV 低位对 UTF-8 中文这种字节模式相近的短串分布差，
  只用低位选槽会长链）。**为什么手写而不是用库**：标准库与 foldhash 的哈希器带随机种子，不能用；blake3 / SHA 这类密码学哈希
  一次几百纳秒、且是为抗碰撞设计的，这里每键要算几千次、只要分布均匀；xxh3 / wyhash 这类非密码学库能用，但任何依赖升级
  悄悄改了算法（或换了默认种子）就会让用户机器上所有 `.qj` 失效，而这个函数总共 12 行、有测试钉死输出值，
  自己写风险最小。若以后碰撞或分布出问题，换成 xxh3 并把 `FORMAT_VERSION` 加一。
- 写文件先写同目录 `.qj.tmp` 再改名；数据文件只整体替换，从不就地修改（mmap 的安全前提）。
- 生成：`cargo run --release -p qingjian-dict-convert -- pack dict --name … --license … --source …` → `dict.qj`，
  `pack lm --name … --license …` → `lm.qj`（`bundle.sh` 在 TSV 比 `.qj` 新时自动重打）。`Dictionary::from_path` 按魔数自动选
  `.qj` / TSV 路径，`BigramModel::from_path`（`.qj`）与 `from_paths`（TSV）分开；输入法与 CLI 有 `.qj` 就用它。
  释义表、emoji 表、英文词表还是 TSV（加载各 20 ms 以内，等有需要再进容器）。
- 格式版本不兼容时 `FORMAT_VERSION` 加一，读旧版的代码按需保留；`Kind` 编号只增不改。

### 候选生成需要整句转换

词级候选（trie 查词库）只能做到「能用」。日常可用的门槛是整句转换：
bigram 语言模型 + Viterbi，加上简拼、模糊音、双拼。没有整句输入，开发者自己都不会切换过来用，
学习功能就没有承载体。

词库与语言模型不自造，见 [landscape.md](landscape.md) 的数据源一节。

### 翻译只是 annotation

翻译 annotation 只对词典词候选有意义。整句引擎产出的候选多数不是词典词
（一整句，或者「的」这种单字），引擎越好，能标注的候选反而越少。

待确认的策略：
- 只给词典词候选标注，句子候选和单字虚词留空。
- 一词多义（开发 → develop / development）只取最高频义项，不展开。

### 翻译数据本地化

翻译走本地查表，运行时不调网络。这样天然满足「翻译不阻塞候选」的约束。
随包的释义表由 `tools/gloss-gen` 用 LLM 离线批量生成（2026-09-04 定的路线，不用有道等网页接口：逆向接口不稳，
攒下来的结果再分发有版权问题；LLM 输出许可干净）：从我们自己统计的一元词频表挑常用词（次数 ≥ 200、不超过 4 个字，约 6.4 万词），
每个词一次请求同时要「词性 + 英文译词 + 日文译词与假名读音」，结果 JSONL 可续跑，`export` 转成 `glossary-en.tsv` / `glossary-ja.tsv`。
释义格式 `词\t词性. 译词\t词性. 译词`，日文译词后 `|假名`；`Sense.reading` 存整词假名，`Sense::furigana()`（Core `candidate::furigana`）用译词里的假名段当锚点把读音对到各段汉字上，
候选窗口按 `開発(かいはつ)する` 显示，假名淡色；纯假名 / 片假名词不注，对不上就整体注在后面。不用罗马音。
CC-CEDICT 表（`dict-convert cedict`）保留为备用来源，覆盖面广但没有词性、释义偏长。
运行时查不到的词走同一条 LLM 通道补进用户目录的个人释义表（**释义兜底**）：Core `GlossFiller` trait 与 `Predictor` 分开注入——联想是「最新请求优先」、
防抖会丢旧请求，兜底恰恰要每个词都问到、慢点没关系。词库词 / 云端词**上屏后**发现随包表没有就入队（只在云联想开着时，发出去的只有那个词），
`qingjian-predict::CloudGlossFiller` 在独立线程攒 1.5 秒或 8 个词发一次请求（提示词与 gloss-gen 同源，只要当前学习语言），问过的本进程内不再问；
壳每秒 `Engine::poll_glosses` 把结果经 `Translator::learn` 写进 `PersonalGlossary`（`user-glossary-<语言>.tsv`，格式同随包表，可手改），
`LayeredTranslator` 个人表优先叠在随包表上，随 `flush_learning` 原子落盘。`Translator::translate` 本身仍不联网。

### 联想与个人模型的边界

- 整句转换里最贵的一步是词图格子查词（每个简拼位置都要在词库里逐音节块收窄）。敲键是增量的，第 n+1 键只新增以它结尾的
  最多 8 个格子，所以格子候选放在 `sentence::SpanCache`（键是格子模式含模糊写法，值是排好截好的 `SpanWord`）跨按键复用；
  候选与词库、用户词、选择次数、个人出现次数有关，Engine 在 commit / `learner_mut` / 换 Learner 时整个清掉。
  拼写纠错的上千个变体先过无分配的 `parser::is_fully_segmentable`，剩下几个才做真正的切分。
- 整句转换的语言模型通过 `LanguageModel` trait 注入（`sentence/language_model.rs`）：`log_prob(previous, word)`，
  模型不认识的词返回 `None`，Core 用词库词频兜底并扣分。`qingjian-lm::BigramModel` 从 `lm.qj`（或 `lm-unigram.tsv` / `lm-bigram.tsv`）
  加载：词表是 arena + 定长条目 + 文件里的开放寻址哈希索引；二元按前词分组成 CSR（`offsets[v]..offsets[v+1]` 是 v 的后继段，
  段内按后词编号二分，一次查找落在一两个缓存行里），P(w|v) = 0.8·c(v,w)/c(v) + 0.2·c(w)/N。
  数据由 `dict-convert bigram` 统计：用青简词库做一元最大概率分词（与词图同一套词表），连续汉字段为句，`<s>` 句首标记。
  词图每格只留词频前 6 个词（有简拼位置的格子留 20 个：`h` 下几十个常用字，留少了句子里要的那个进不来），每个位置束宽 8。
  简拼位置就是前缀模式（`SyllablePattern.complete = false`），词库层不区分；每条路径覆盖的简拼位置相同，不需要额外罚分。
- **个人 n-gram**（`sentence/user_ngram.rs` 的 `UserNgram`，由 Learner 持有、`Learner::user_ngram()` 暴露）：
  上屏的词序列转移计数，二元 (前词, 后词) 与三元 (前二词, 前词, 后词) 一起记（整句按路径上的词逐条记，连续选词也记；
  标点、透传、回车上屏拼音、切应用打断链，下一个词按句首 `<s>` 记，句首词不记三元）。上文是 `sentence::Context`（前一个词 + 再前一个词），
  Engine 的 `CommitChain` 记最近两个上屏的词；Viterbi 不扩状态，前二词取前驱节点的回指（它那条最优路径上的前一个词），是近似。
  打分时与静态模型（只看前一个词）插值：P = (1−μ)·P_静态 + μ·P_个人，μ = c(v)/(c(v)+8) 封顶 0.5，前词没见过就不插值。
  P_个人 先算二元 P₂ = 0.8·c(v,w)/c(v) + 0.2·c(w)/N；这对上文 (u,v) 见过时再套一层绝对折扣的三元
  P₃ = max(c(u,v,w) − D, 0)/c(u,v) + D·N₁₊(u,v,·)/c(u,v)·P₂（D = 0.75），没见过的接续只拿回退的份额，(u,v) 没见过就是 P₂。
  三元不训练、不平滑参数，就是在线计数；它分辨的是二元混在一起的接续（「我想 → 去」与「不想 → 要」）。
  封顶保证没见过的接续最多打折、不会被压死；K = 8 让一次误选翻不过强 bigram，选两次才翻。
  个人出现次数也参与词图每格的前 6 选择，保证用户常用的同音词进得了格子。二元 + 三元超过 20 万条时所有计数减半。
  持久化在 `qingjian-learning` 的 `user-ngram.tsv`：三列 `前词\t后词\t次数` 是二元，四列 `前二词\t前词\t后词\t次数` 是三元，旧的三列文件照读。
  撤销（退格删光重选）、删词（`forget_word`）、减半都同时覆盖二元与三元。
- **自动造词**：用户自己连着选出的两个词（不是整句路径里的），合起来不超过 4 个字、词库与用户词里都没有，
  且这条转移已记够次数（同一段拼音里连着选的两次，分两段打的三次），就记成用户词并记一次选择。
  同一段拼音里自选一个词之后剩下的部分走整句候选（`jidiaole` 选 挤、剩下 掉了 按空格）时，接缝处第一个词的转移按自选记双份，
  但不参与两词造词（我 + 的… 这种接缝太常见、转移计数早就够了，回放里会把 我的 造成用户词，之后它就得和 沃德 按选择次数比）。
  另外一段拼音分几次选完（`CommitChain` 记着这段里上屏的每个词）时，合起来的文本按「整段字母 → 合成词」记一次选择（`user-choices.tsv`），
  记到两次且词库里没有、不超过 4 个字就造成用户词，下次整段打出来它靠选择次数直接排第一：这是「用户手动拼了一遍整句」最直接的信号，
  比等个人 n-gram 一份一份累到翻过静态模型快得多（「挤掉了」靠 n-gram 要选四次）。
  词级排序也用同一个语言模型：候选得分是 `log P(词 | 上一个上屏的词)`（个人 n-gram 插值，上文是链上的两个词）+ 封顶的选择次数加分，词库词频只做预选和兜底，
  所以 `ba` 在「做了」后面出 吧、句首看模型；同一输入串下选过的词（`user-choices.tsv`，键是候选覆盖的那段字母）排在得分之前，
  `mgs` 选过 美国式 下次就是首选；切分里非末尾简拼少的优先（`kaifa` 按 `kai fa` 读的 开放 压过按 `kai f a` 读的 开放啊）。
  拼写纠错（Core `correction`）两路：整段一处编辑的变体按噪声信道挑（纠正后整句得分扣编辑代价仍高于原样才纠），候选按纠正后的拼音出，
  消耗长度按编辑换算回原串；词图里的敲错边（`correction::typo`，每个完整音节的一处敲错变体当带代价的位置写法，与模糊音同一套 `Expanded`
  多写法机制，`SpanWord::penalty` 进路径得分，`Conversion::penalty` 让 Engine 知道路径不是原样读的）管「音节都合法、整句不通」的输入，
  只进整句词图不进词级候选。接受的纠正按原输入串记选择，回车原样上屏的串记 `<raw>` 以后不纠；两路接受的 (敲的, 要的) 音节对都记进个人敲错表
  （`Learner::record_typo`，`user-typos.tsv`），那条边与整段编辑的代价按次数打折（`TypoCosts::discounted`）。判断带缓存（按作用域），commit / take_raw 复用 query 的结果。
  候选音节对回敲的字母（消耗、记敲错）用 `Engine::align`，模糊音命中也走它。
  退格撤销（`LastCommit`，Engine 留最近 4 次上屏 `recent_commits`）：上屏后壳把组句外的退格告诉 Engine（`note_backspace`），退格从最近一次往前数，
  一次上屏的字删光了就候着；接着重打其中一段拼音（或其前缀）选了别的词，就把那次记的选择次数、输入串选择、词转移、整段合成词的选择全部退回（`Learner::unrecord*`）。
  删掉「沃德 书」两个词重打成「我的 书」也认得出（日志里这种错法一天十几次，以前只看最后一次上屏，一次都没撤回，错词越选越靠前）；
  重打后选的还是同一个词只把记录丢掉；重打的拼音谁都对不上就当在改别处，全忘掉；删得比记着的几次加起来还多也全忘掉。
  标点、英文词、原样上屏这些没学习的上屏也留一条只有长度的记录，退格数过它们才能数到更早的词。
  整句候选与某个词候选文本相同时（用户词、或按别的读音对上的词）不重复插，但把那个词提到整句该在的位置：整句转换认定的最好读法不该被词级排序（别的词选过更多次）压在后面。
  云端词学成用户词时读音以用户敲的拼音为准（能切成与字数相同的完整音节、每个又是那个字的读音时），否则用模型给的但逐字核对词库读音，核不过不学：
  模型把「我的」读音给成 `wo di` 之后，这条用户词按错读音出现在候选里，把正确的整句候选「我的」顶掉、自己又排在「沃德」后面，就是这么来的。
  用户点选的转移记双份（`EXPLICIT_TRANSITION_WEIGHT`），整句路径里顺带的记一份：整句是模型自己算的，按空格接受会把它喂回模型形成回声，
  用户明确改选一次就要能压过去。选择次数在整句路径上的加分取对数并封顶（`viterbi::WEIGHT_CAP`），只管同音词偏好，不许它抬起拆分路径。
- 联想通过 `Predictor` trait 注入，与翻译一样是异步补充：**不阻塞候选、不重排已有候选**，超时即丢。
  网络实现放 `qingjian-predict`，Core 不依赖它，也永远不联网。
  请求带 `reasoning_effort`（配置 `[predict] reasoning_effort`，缺省 `none`）：DeepSeek V4 这类默认思考的模型不关会把 token 预算花光、正文为空；
  密钥环境变量缺省 `QINGJIAN_API_KEY`（`api_key_env` 可改），名字跟产品不跟供应商，因为 `base_url` 本来就可以指到别家。
  接口是非阻塞的 `submit` / `poll`：网络 crate 自己开后台线程做防抖、缓存、超时；壳用定时器轮询结果；
  Engine 给请求编号，只认最新序号的结果。观察窗口在 Core 里裁剪，壳给再多也只发这么多。
- 只在组句中联想（拼音 ≥ 2 个字母、停键 300 ms 后），一次请求两种产物：
  **云端词**：用户最可能想打的词，带正确全拼；模型可以纠错（`zhgdoima` → 这个东西吗），所以 Core 用容错校验
  （`prediction/fuzzy.rs`：字母与全拼的编辑距离，简拼不算错，容错数随长度增长，不到 4 个字母不容错）而不是逐音节精确匹配；
  通过的插到候选第二位起（本地首选不动、其余相对顺序不动），云朵标记，数字键选，上屏吃掉整段拼音；
  词库里没有的记成**用户词**（`user-words.tsv`，与主词库同格式），下次本地直接出且靠 weight 排前。
- **个人英文词**（`Learner::learn_english` / `user_english`，`user-english.tsv`）：回车 / 英文模式直通原样上屏的、像英文词的字母串
  （中文模式下要求切不成完整拼音）和选中的英文候选，组成一张小 `WordList`，与随包英文词表一起出英文候选、排在前面。见 candidate-ui.md。
  **整句补全**：以这个词开头的完整说法（`suoyiwoxiangq` → 所以我想去吃饭），画在 preedit 右侧，Tab 接受。
- 上下文只来自应用（IMK `attributedSubstringFromRange:`）；终端、微信这类给不出的就只靠拼音。
  **本地输入历史不当上下文**：它是跨应用拼起来的碎片，用它联想出来的全是噪音。
  上屏之后不联想：没有拼音约束的下文联想每次上屏多发一次请求，纯靠猜，已删除。
- 联想不限语言：模型按光标附近文本的语言续写。
- **问字模式**（`?` 开头）复用同一条通道：`PredictionRequest.kind = Question`，只带问题拼音（不带应用上下文、不要整句），
  `qingjian-predict` 按 kind 换系统提示，回复是 `answers`（字 / 短答案 + 带声调读音，放 `CloudWord.reading`）；
  Core 在问字模式下不做拼音校验、本地不出候选。
- 发往云端的上下文默认关闭；开启后 Secure Input 绝不发送，前后观察长度可配置。
  上下文优先从应用读（IMK `attributedSubstringFromRange:`），读不到退回 Core 的本地输入历史（内存环形，可清除）。
- 个人化优先用在线 n-gram，神经模型只做重排与离线联想，且要过评测门槛（见 roadmap Phase 7）。
- 两者都依赖本地输入历史，历史必须可查看、可清除。

## 平台层的技术决定

### macOS：IMK

- 使用 `objc2` + `objc2-input-method-kit`。
- 候选窗口自定义 NSPanel（竖排 / 横排可配），不用 IMKCandidates。定位取光标所在那块屏幕的可见区域：贴光标行下方，放不下放上方，左右不出屏；
  应用给不出光标矩形时以鼠标位置为准。
- `apps/macos/src` 按职责分目录，模块文件只做 `mod` 声明与 re-export：
  `main.rs` 初始化 host、建 IMKServer 并跑 NSApplication；
  `host/` 是进程级单例（一个 Engine + 一个候选窗口，`thread_local`，IMK 回调全在主线程；`mod.rs` 放结构体与 `with`，`init.rs` 启动加载、`config.rs` 热加载、`settings.rs` 菜单 / 偏好设置动作、`dictionaries.rs` 词库管理、`cloud.rs` 云端、`diagnostics.rs` 诊断与日志、`presenting.rs` 呈现），
  `host/` 下是会话状态 `session.rs`、联想轮询定时器 `predict_monitor.rs`、配置文件监视与定时落盘 `config_watch.rs`、
  短提示 `notice.rs`、翻译选中文字的任务 `translation_job.rs`、附加词库装配 `extra_dictionaries.rs` / `dictionary_info.rs`；
  `imk/`：`controller.rs` 用 `define_class!` 继承 `IMKInputController`（类名 `QingjianInputController`，
  与 Info.plist 的 `InputMethodServerControllerClass` 一致），只做按键 → Engine、Engine → 窗口；
  `client.rs` 用 `msg_send!` 封装 IMKTextInput（`setMarkedText:` / `insertText:` /
  `attributesForCharacterIndex:lineHeightRectangle:` 取光标矩形）；`modifiers.rs` / `secure_input.rs` 查系统状态；
  `candidates/`：`window.rs` 是非激活浮动 NSPanel（level 101、CanJoinAllSpaces、忽略鼠标），
  `view.rs` 自绘顶部拼音行与候选（竖排 / 横排两套画法），`theme.rs` 集中字体颜色间距，`row.rs` 把 Candidate 转成展示片段，
  `preedit/`（`mod.rs` / `segment.rs` / `style.rs`）是拼音行的分段模型（由 Core 的 `MarkedSegment` 转来），`frame.rs` 是一帧的数据；
  `menubar/`：`indicator.rs` 是菜单栏的中 / 英 NSStatusItem（输入源图标没法动态换，只能自己放一个），
  `menu.rs` / `action.rs` / `target.rs` 是输入法菜单；
  `preferences/`：偏好设置窗口（`window.rs` 手排控件、`layout.rs` 逐页排版、`panel.rs` 关窗时切回激活策略、`setting/`（`Setting` 与 `SettingValue`）控件 ↔ 配置项、
  `target.rs` 一个 `changed:` 选择器、`key_recorder.rs` 快捷键录制按钮、`usage_page.rs` 「统计」页（数字格子与「几本《某书》」文案）、`about.rs` 「关于」页文案、`edit_menu.rs` 只有编辑项的主菜单、`file_dialog.rs` 导入词库的打开面板）；
  `app/`：`paths.rs` 定位 `.app/Contents/Resources/`（词库、随包领域词库 `dicts/`）与 `~/Library/Application Support/Qingjian/`（用户数据），
  `settings.rs` 是配置文件的运行时状态，`logging/` 只写 `~/Library/Logs/Qingjian/`（自己的 `LogFile` 按天分文件、留 7 天、被删重建），`bundle.rs` 读 Info.plist，
  `input_source.rs` 是 `qingjian-macos --register`：走 Carbon TIS（`TISRegisterInputSource` + `TISEnableInputSource`，再起子进程 `--finish-register` 回读 `IsEnabled` 并 `TISSelectInputSource`，隔 3 秒二次确认）把 `.app` 注册成输入源并切成当前。两个坑：TIS 状态按进程缓存，本进程回读永远是旧值，只有新进程看得到；刚换过包的 3–5 秒内系统重扫会把刚启用的记录顶掉，所以要二次确认并启用。
- **打包与分发**（`apps/macos/scripts/bundle.sh`）：版本号来自 workspace `Cargo.toml`，构建号是提交数，打包时用 PlistBuddy 写进 Info.plist。
  `--install` 装到 `~/Library/Input Methods/`（开发用）；`--pkg` 做 `target/pkg/Qingjian-<版本>.pkg`：`pkgbuild` 组件包装到
  `/Library/Input Methods/`（macOS 输入法的惯例位置，需要管理员密码；组件描述里关掉 bundle 重定位，否则会装到机器上同 id 的旧副本那里），
  postinstall 杀旧进程并 `launchctl asuser <uid> sudo -u <登录用户> qingjian-macos --register`（安装器是 root，输入源是每用户的），
  `productbuild` 套上欢迎页 / 许可证（`LICENSE`）/ 结束页（`apps/macos/pkg/`）。签名与公证全由环境变量决定：
  `QINGJIAN_SIGN_IDENTITY`（Developer ID Application，开 hardened runtime）、`QINGJIAN_INSTALLER_IDENTITY`（Developer ID Installer）、
  `QINGJIAN_NOTARY_PROFILE`（notarytool keychain profile，设了就公证并 staple）；没设就 ad-hoc 签 `.app`、pkg 不签，
  测试者要在「隐私与安全性」里点「仍要打开」。卸载脚本 `uninstall.sh` 随包放在 Resources。二进制只有本机架构，Intel 要另打。
- 配置只有一条通路：`Host::apply_config` 把当前 `Config` 推给 Engine（模糊音、模式键、Predictor 重建、释义表切换）与界面
  （每页候选数、翻页键、外观、☁︎ 标识、菜单勾选、设置窗口控件）。启动、菜单开关、设置窗口、`host/config_watch.rs`
  每秒一次的 mtime 监视全都走它；三个入口都只写 `config.toml`，不各存一套状态。解析失败沿用上一份，错误显示在菜单与设置窗口里。
  按键走 `inputText:client:` +
  `didCommandBySelector:client:`，不用 `handleEvent:`。**组句期间 `didCommandBySelector:` 对不认识的
  选择器也要返回 YES**：返回 NO 会让应用自己处理方向键，应用一动光标就把 marked text 丢了，
  而我们的缓冲区和候选框还在（2026-09-03 踩过）。
- `define_class!` 的类在首次调用 `class()` 时才注册到 ObjC 运行时，而 IMKServer 初始化时就按
  Info.plist 的类名查找，找不到会**静默退回基类**，症状是按键全部透传、像在打英文。
  必须先 `QingjianInputController::class()` 再建 IMKServer（2026-09-03 踩过）。
- `define_class!` 里返回 `bool` 的方法体内不能 `return`（宏会把返回类型换成 ObjC `BOOL`），
  逻辑放到 inherent impl 里，宏内只做转发。
- Info.plist 约定：bundle id 是 `app.qingjian.inputmethod`（域名 qingjian.app 的反写 + 产品，其他平台外壳共用 `app.qingjian.` 前缀），`TISInputSourceID` 与它相同，`InputMethodConnectionName` 必须是 `<bundle id>_Connection`；
  `LSBackgroundOnly = true`；ad-hoc `codesign` 之后 Apple Silicon 才会加载。
- IMK 无法通过 `cargo run` 验证：需要打包成 `.app`、装到 `~/Library/Input Methods/`、
  注销或重启输入法进程才会生效。Core 的验证靠 CLI 测试工具和单元测试，不依赖跑起真实输入法。
- 已知需要单独处理的场景：Secure Input 字段、沙盒应用、Electron 与 Terminal 各自的 marked text 行为。

### Windows：TSF

- TSF DLL 会被加载进每一个应用进程，核心逻辑必须放在进程外。
  采用 Weasel（WeaselServer）和水杉（Server 进程）相同的结构：DLL 只做 IPC，Rust Core 跑在独立进程里。
- 使用 `windows` crate 的 COM `implement` 宏。
- TSF 是公认最难的输入法 API，工时预期要按整个项目一半来估。

**已落地（骨架）：**

- **IPC 协议**：`qingjian-platform::protocol`，Server ↔ DLL 两端共用、全部 serde。`ClientMessage`（DLL → Server：
  开 / 关会话、按键、上屏、回上下文、回选区、报中英模式）与 `ServerMessage`（Server → DLL：按键结果、上屏结果、异步重绘、请求上下文、请求选区）；
  失焦 / 停用时 DLL 发 `Commit`，Server 回 `Committed { text }`（缓冲区原样交出，对应 macOS 的 `commitComposition`），
  DLL 用最近收键记下的 `ITfContext` 经编辑会话落进文档；应用强行终止组句（`OnCompositionTerminated`）时拼音已被框架定成普通文本，
  DLL 只记「Server 缓冲过期」，下次说话前先 `Commit` 并丢掉交出的文本，不再插一次。
  中英模式：**Windows 与 macOS 机制不同**。macOS 用 Caps Lock 当中英切换键；Windows 按本地习惯，单击 Shift 在中 / 英间翻转。
  单击 Shift 的判定在**击键 sink** 里（`com/key/shift.rs`，喂 `OnTestKeyDown` / `OnTestKeyUp`：按下 Shift 到抬起之间没有别的键插进来就是一次单击；
  微软 SampleIME 的 `OnTestKeyDown` 同样处理 VK_SHIFT，sink 收得到独立修饰键）。之前用线程级 `WH_KEYBOARD` 钩子判定，但钩子**看不到被 TSF 吃掉的键**
  （msctf 在队列层把它们改成 WM_NULL），Shift + 数字（删候选 / 第二译词）会被误判成单击而切换模式，2026-09-11 真机确认 sink 收得到 Shift 后钩子已删。
  「翻译选中文字」的快捷键（缺省 Ctrl+Alt+T）**登记成 TSF 保留键**（`com/key/preserved.rs`，`ITfKeystrokeMgr::PreserveKey` → `OnPreservedKey`）：
  带 Alt 的组合是系统键、不经击键 sink（真机 `OnTestKeyDown` 里从没出现过），保留键由 TSF 在应用之前匹配，UWP 里也一样；组合激活时从
  `config.toml` 读一次（AppContainer 读不到用户目录时用缺省），命中后当作那个组合键转发给 Server 走原有的 `RequestSelection` 流程。
  DLL 记 `english_mode` 持久状态；任务栏的中 / 英指示器靠 `GUID_LBI_INPUTMODE` 语言栏按钮渲染
  （`com/mode/button.rs`，图标现画「中」/「英」，第三方 TIP 单写转换模式 compartment 不出这个指示器），另外顺带写一份转换模式 compartment
  （`GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION` 的 `TF_CONVERSIONMODE_NATIVE` 位，`com/mode/mod.rs`）。这条 compartment 还**反向同步**：激活时对它挂
  `ITfCompartmentEventSink`（`com/mode/conversion.rs`），用户点任务栏中 / 英（或别的输入指示器途径）改了转换模式时 `OnChange` 读回 `NATIVE` 位、与当前
  `english_mode` 不同才翻转（相同即我们自己写的那次，忽略以防回环），翻转顺带走 `update_mode_indicator` → 悬浮状态条也一起同步；Caps Lock 只管大小写，
  亮着无论中英模式都直接出大写英文（微软拼音式）。`KeyModifiers` 因此带 `caps`（大小写）与 `english_mode`（持久模式）两个非物理位，
  字母大小写按 `shift XOR caps`。Router（`dispatch/key/input.rs`）里 `english = caps || english_mode`、候选只在 `english_mode && !caps && 应用允许` 时给，
  `[general] english_candidates` 关着就是纯直通。macOS 的「先上屏、再把这个键交给应用」在 Windows 上会乱序
  （放行是同步的、上屏走异步编辑会话），所以组句中的空格 / 标点 / Shift 大写字母改成吃掉，连同上屏文本一起插入；
  `[apps] english_candidates_off` 按应用关闭：应用标识在 Windows 上是宿主进程的 exe 文件名（DLL 加载在应用进程里，`GetModuleFileNameW(NULL)`
  取到就随 `OpenSession { app }` 报一次，Server 每会话记下，收键时按当前会话查），缺省名单分平台（`AppsConfig` 的两份常量与配置模板的 `[apps]` 一节都按 `cfg(windows)` 选），
  经典控制台的窗口属于 `conhost.exe`、Windows Terminal 是 `WindowsTerminal.exe`；
  `[shortcut]` 的修饰键 + 数字（译词上屏 / 删候选，`dispatch/key/shortcut.rs`）：配置里的 `Modifiers` 按 option→Alt、control→Ctrl、command→Win
  落到 `KeyModifiers`，Router 按键码认数字、去掉 Caps 位后与配置比；DLL 见 Ctrl / Alt / Win 仍一律放行，只有组句中的修饰键 + 数字送 Server 判，
  没配到的 Router 回 Passthrough。删候选的那句反馈（「已删除…」/「没什么可删」）随下一帧的 `Frame::notice` 下发，自绘候选窗画在拼音行下方、
  显示到下一次按键（`dispatch/key/shortcut.rs` 填、`handle_key` 开头清；对齐 macOS 画在拼音行右侧的短提示）。
  **翻译选中文字**（`[shortcut] translate_selection`，缺省 Ctrl+Alt+T）：不在组句、云服务开着时按下它，Server 回 `RequestSelection`
  （替代那次按键的常规结果），DLL 起一个**异步只读编辑会话**（`com/edit/selection.rs`，`GetSelection` + `GetText` 读选中文本、上限 500 字、`GetTextExt` 量屏幕矩形）
  回 `Selection { text, rect }`；Server 走 Core 的 `request_translation`（`PredictionKind::Translate`，双向，译文走 `sentence`），
  在**自绘候选窗**里以选区矩形为锚显示单条译文（先「翻译中…」，云端回来再换），评审态吃走所有键：回车 / 空格接受、Esc 保留原文、其余键放弃并交回应用。
  替换选区不另加协议——接受时 Server 把译文当 `commit` 回给 DLL，DLL 无活动组句时 `InsertTextAtSelection` 正好替换当前选区。DLL 用 `Shared::translating` 标志让评审期吃键、轮询定时器照常拉云端译文、失焦收窗。
  一次要绘制的状态是 `Frame`（preedit 分段 + 候选页 + 可选的整句补全 `sentence` 与删候选提示 `notice`，后两者不参与 `Frame::is_empty`），preedit 用 `PreeditSegment`（Core `MarkedSegment` 的可序列化镜像，
  协议不耦合 Core 内部枚举），候选直接嵌 `qingjian_core::CandidateList`。同词干类型收进子目录：`key/{event,outcome}`、`frame/preedit/{kind,segment}`。
- **Server 进程**：`apps/windows/server`（package `qingjian-windows-server`，bin `qingjian-server`）。`dispatch::Router` 按 `SessionId` 分派多会话（Windows 一个 Server 服务多个应用进程，
  每会话各持组句状态，不同于 macOS 的进程级单例）。会话开 / 关、按键与上屏、Engine 装配、命名管道传输（`\\.\pipe\qingjian`）都已跑通，Windows 上端到端测过。
- **候选窗口（Server 进程自绘 + uiAccess）**：候选窗从前在**应用进程内的 DLL** 自绘，普通置顶窗被微软商店 / 任务栏搜索这些**更高 z-band** 的宿主盖住。现改由 **Server 进程**自绘（`server/src/ui/`：一条专用 UI 线程注册窗口类 + 建 GDI 分层窗 + 跑消息循环，HWND 只在该线程碰；工人线程经 `Sender<UiCommand>` + `PostThreadMessageW(WM_APP)` 把「显示(`Frame`+屏幕矩形) / 隐藏」marshal 过去；进程级 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` 按物理像素对齐应用报来的矩形）。DLL 只量光标屏幕矩形（`GetTextExt`，退鼠标）发 `PositionCandidates{rect}`，并在组句于 DLL 侧结束（应用终止组句 / 断线，`OnCompositionTerminated` 这条 Server 无从知晓）时发 `HideCandidates`；Server 握着 `Frame` 直接自绘，云端异步更新也直接刷自己的窗、不回传 DLL（渲染代码——词性 + 译文 + 分页 + 柔和阴影，对齐 macOS——整块从 DLL 搬到 Server）。**盖过高 z-band 宿主**靠 Server exe 的 `uiAccess="true"` manifest（`server/build.rs` 用 embed-manifest 嵌）+ 代码签名 + 装 Program Files 三者齐备（`SetWindowPos(HWND_TOPMOST)` 才自动升进 UIAccess 高带）：开发自签 + 本机受信任根（`installer/sign-local.ps1`），发版换 Certum 开源代码签名证书；uiAccess exe 不能 CreateProcess 拉起（报 740），装完 / 登录都走 ShellExecute（安装器完成页 `ShellExecAsOriginalUser` + `{commonstartup}` 启动快捷方式由 Explorer 拉起才授 uiAccess，故不用计划任务）。候选窗每显示一页，Server 调 `Engine::note_displayed`（收窗传空）告知当前页——生词「看到轮次」据此推进、橙色标记满 `FRESH_UNTIL` 轮才毕业，对齐 macOS 壳的 `render`。
- **悬浮状态条（Server 进程自绘，可拖动 / 记位置）**：桌面上常驻的小浮窗，显示当前中 / 英（开着双拼时附方案名），与任务栏的中 / 英指示器（语言栏按钮）并存。跟候选窗**同一条 UI 线程**、复用同一套分层窗口合成器（`server/src/ui/layered/`：圆角背景 + 四周柔和阴影，从候选窗的 `surface.rs` 抽出来两边共用）与主题（字体 / 配色 / DPI / 深浅）；自己一个窗口类与窗口过程（`server/src/ui/status/`）：三格 `[中 / 英][，。/ ,.][⚙]`：按下鼠标先 `DragDetect`，挪出阈值就交给系统移动循环（`WM_NCLBUTTONDOWN` + `HTCAPTION`，结束时 `WM_EXITSIZEMOVE` 报新位置），没挪就是点击、按 x 落进哪格；`WM_MOUSEACTIVATE` 回 `MA_NOACTIVATE` 点它不抢应用焦点；窗口过程按 HWND 从 thread_local 表查到对象。点格 / 拖动结束经 `StatusEvent`（`dispatch/status/`）投回工人线程（工人循环收的是 `ipc::Work`：DLL 消息或状态条事件），Router 写回配置（`[status_bar] x/y`、`[general] full_width_punctuation`，热加载再读回）；齿轮由 UI 线程直接起设置程序。中英模式只在 DLL 侧（单击 Shift 翻转），DLL 在切换 / 激活 / 获焦时用 `ClientMessage::ModeChanged { english }` 把当前会话的模式推来（`com/service/mode.rs::refresh_mode_indicator` 的单一咽喉点）；状态条上点「中 / 英」时 Server 只能记下目标模式（`pending_mode`）等 DLL 来取：DLL 的轮询定时器在没组句、本线程前台时每几拍发 `SyncMode`，`ModeSync { english: Some(_) }` 就切并回报 `ModeChanged`。状态条**常驻桌面**，只跟「当前输入法是不是青简」走：第一次 `ModeChanged` 显示，DLL 挂 `ITfActiveLanguageProfileNotifySink`（`com/profile.rs`）在别的 TIP 被激活时用一条临时连接发 `ImeSwitched` 收起（此时自己已被停用、会话连接已关），应用退出（`CloseSession`）不收。双拼方案 Server 从自己的 `[general] shuangpin` 配置知道，不必带。开关与记住的位置在 `[status_bar]`（`enabled` / `x` / `y`），热加载即时生效；uiAccess 高 z-band 与候选窗同进程天然继承。参考微软水杉的 FTB 形态（`~/Desktop/MSIME-Windows`，它用 D2D + DirectComposition 且不记位置），落地时选沿用本项目已有的 GDI 分层窗那套以保持视觉语言一致、并加了位置持久化。
- **帧编解码**：长度前缀 JSON 帧的 `read_message` / `write_message` 与缺省管道名放在 `qingjian-platform::protocol`，Server 与 DLL 共用（DLL 不必依赖整个 Server 库）。
- **TSF DLL**：`apps/windows/tsf`（package `qingjian-windows-tsf`，`cdylib`，产物 `qingjian_tsf.dll`，依赖官方 `windows` crate 的 COM `implement` 宏）。「引擎层」不是 Engine 而是连 Server 的**管道客户端** `EngineClient`（平台无关、可端到端测）；
  COM 层：`DllGetClassObject` → `IClassFactory` → `#[implement(ITfTextInputProcessor, ITfKeyEventSink, ITfDisplayAttributeProvider)]` → `Activate` 挂击键 sink + 登记翻译保留键 + 语言栏中英按钮 + 连管道 → `OnKeyDown` 转发按键、经异步编辑会话（`TF_ES_READWRITE`，不带 SYNC）写组句 / 上屏；`DllRegisterServer` 写 InprocServer32 并经 `ITfInputProcessorProfiles` / `ITfCategoryMgr` 注册文本服务与各能力类别。
  组句拼音的**内联下划线**（对应 macOS marked text 下划线）走 TSF 显示属性协议（`com/display_attribute/`）：注册 `GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER` 类别 + 一个自定义显示属性 GUID（细实线、`TF_ATTR_INPUT`），
  `ITfDisplayAttributeProvider`（实现在 TextService 上）把 GUID 对应的 `TF_DISPLAYATTRIBUTE` 交给系统；收键写组句时用 `ITfCategoryMgr::RegisterGUID` 把 GUID 换成 atom，`SetValue` 进组句范围的 `GUID_PROP_ATTRIBUTE` 属性，宿主据此在拼音底下画线。
  收键与运行细节记进 `%LOCALAPPDATA%\Qingjian\tsf.<日期>.log`（按天一个文件、留 7 天，与 Server 一致；多进程追加同一文件）；候选窗口不再由 DLL 自绘（已搬到 Server 进程，见上「候选窗口」），DLL 侧只做 preedit 内联 + 上报光标矩形；云联想已接。
- **交叉编译验证**：`qingjian-core` / `-dictionary` / `-format` / `-lm` / `-platform` / `apps/windows/{server,tsf}` 已能
  `cargo check --target x86_64-pc-windows-gnu` 通过（借此修掉 `qingjian-format` 里 unix 专有的 `Mmap::advise` 未 `cfg` 的移植 bug）；
  本机只 `check`，真正编译在 Windows 机器上做（`qingjian-neural` 的 candle 后端在 Windows 走 CPU，已接进 Server，见下「本地整句模型」）。
- **本地整句模型（Server 进程，与 macOS 的 `host/model.rs` 对齐）**：`server/src/dispatch/rescore/`。启动时 `find_model_dir`（用户目录 `%APPDATA%\Qingjian\model\` 优先，否则随包 `data\model\`）；`[model] enabled` 开着就起线程加载并预热（`ModelLoader`），下一次按键 / tick 接上 `set_async_sentence_scorer`。
  Server 没有定时器：缓冲变化后 `schedule_rescoring` 起防抖，工人循环 `recv_timeout(router.next_tick())` 按 `RescoreState` 的节拍醒来（防抖 80 ms → `request_rescoring`；然后 20 ms 一次 `poll_rescoring`，最多等 2 s），DLL 组句期间每 80 ms 的 `Poll` 也顺带 `tick`。分到了重查一次、重建候选布局（云端词与整句补全留着）、由 Server 自绘的候选窗直接重画，DLL 下一次 `Poll` 拿到新帧更新内联 preedit；翻过页 / 动过高亮不动。热加载 `[model]` 变了才重载 / 卸载。
  前文：DLL 在**起组句的那次读写编辑会话**里顺手读选区起点前 64 个 UTF-16 单元（`com/edit/surrounding.rs::text_before_caret`，拼音还没插进去、不用再开一次会话），随 `ClientMessage::Surrounding` 单向送来；密码框（`GUID_PROP_INPUTSCOPE` 含 `IS_PASSWORD`）/ 读不到不发。Server 收到（`set_surrounding`）就 `set_rescoring_context`、重查一次按新前文记下要打分的文本并重新计时；组句结束前文作废。没有前文退回本会话历史。协议版本 3。
- **版本与发布**：各平台壳版本号独立（见 `docs/notes/release.md`）；`apps/windows/server/Cargo.toml` 写死自己的 `version`，
  将来的发布标签用 `windows-v<版本>`，与 macOS 的 `macos-v<版本>` 互不影响（`qingjian-windows-tsf` 是同一 Windows 产品的另一半，各自 `Cargo.toml` 记版本；两个 package 同放 `apps/windows/` 下，是一个产品的两个产物——不合成一个 crate，因为 DLL 不能带 Engine 的依赖树）。

### Linux：IBus / Fcitx

- IBus 走 D-Bus（`zbus`），纯 Rust 即可。
- Fcitx5 需要一层 C++ shim(Maybe)。
