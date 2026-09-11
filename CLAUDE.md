# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 仓库现状

macOS 输入法已自用（HEAD 见 git），正在给测试者打包（pkg 已做，签名等 Developer ID 证书）。各部分：

- `crates/qingjian-dictionary`：词库（TSV 解析或 `.qj` mmap），键按字节序排好，查询逐音节位置二分收窄（简拼位置按音节块跳扫），
  `lookup_pattern`（≥ 模式长度）与 `lookup_exact`（正好等长）同一套实现。
- `crates/qingjian-core`：`composition` / `parser` / `correction`（拼写纠错：整段一处编辑的候选纠正 + `typo` 音节级敲错变体表，后者进整句词图当带代价的边）/ `candidate` / `ranking` / `shortcut` / `sentence` / `fuzzy` / `shuangpin`（双拼：四套方案键位表、键 → 全拼解码与消耗换算）/ `emoji` / `english`（英文模式候选）/ `engine`（`query::EnglishTail`：句末英文词并入整句，`woxiangxuehaorust` → 我想学好rust，尾段也像拼音时按分数与拼音读法比）/ 
  `Engine` 是对外唯一门面，`Translator` / `Learner` trait 在 `engine` 模块；词库是「主词库 + 附加词库（`set_extra_dictionaries`）+ 用户词」的列表。
- `crates/qingjian-translate`：`Glossary`，本地 TSV 释义表（词性 + 译文）；`LevelTable`，词汇等级表（`assets/levels/levels-{en,ja}.tsv`，CEFR A1–C2 / JLPT N5–N1，
  `uv run tools/corpus/levels.py` 从 `data/levels/` 的原始 CSV 生成，来源与许可见 `assets/levels/README.md`），「统计」页按级数词汇用，不进候选。
- `crates/qingjian-learning`：`FrequencyLearner`，用户选择次数（`user.tsv`）、按输入串记的选择（`user-choices.tsv`，词级排序里同输入串选过的优先）、用户词（`user-words.tsv`，主词库同格式，
  Engine 与主词库一起查）、个人英文词（`user-english.tsv`，回车原样上屏的英文词与选过的英文候选，与随包英文词表一起出候选且在前）、个人敲错表（`user-typos.tsv`，接受过的 (敲的, 要的) 音节对，词图敲错边与整段纠错的代价按它打折）与个人 n-gram（`user-ngram.tsv`，Core `sentence::UserNgram`，二元 + 三元在线计数，整句转换与词级排序里与静态模型插值；Tab 接受的云端整句按 `sentence::segment_text` 切词后也记；
  连着选出的两个词记够次数自动造词进用户词，一段拼音分几次选完的合成词记两次也造）；`InputLog` 是输入日志（`input-log.jsonl`，每次上屏一行：敲的键、切分、看到的前几个候选、选了第几个、来源、纠错、撤销，
  Core `InputLogger` trait 的落盘实现，`[general] input_log` 缺省开，只写本机，给离线回归评测与个人模型用）；`UsageStats` 是输入统计（`usage.tsv`，按天记汉字 / 中文词 / 英文词 / 上屏次数，
  Core `UsageMeter` trait 的实现，Engine 每次上屏 `Usage::of_text` + 按来源定词数，整句按 `segment_text` 切词数；与输入日志无关，偏好设置「统计」页显示，`book_scale` 折成几本《某书》）；
  `VocabularyBook` 是词汇记录（`user-vocab.tsv`，Core `VocabularyTracker` trait 的实现：学习语言的每条译词看到过几轮 / 上屏过 / ⌥+数字 打出过几次；
  Engine `annotate` 据此填 `Sense::fresh`，看到轮次不到 `FRESH_UNTIL` = 3 的译词壳里画橙色；「看到」按上屏那一刻屏幕上那一页算，壳每次画完 `Engine::note_displayed` 告知当前页）。
  各表落盘走 Core `storage::write_atomic`（临时文件 + fsync + 改名），加载按行容错（坏行警告跳过，真读不了壳退回内存学习），
  壳激活期间每 60 秒 `Engine::flush_learning`；IMK 回调边界 `imk::catch_panic` 拦 panic、缓冲区字母原样上屏（见 architecture.md「崩溃不丢」）。
- `crates/qingjian-predict`：`CloudPredictor`，`Predictor` trait 的网络实现（async-openai，OpenAI 兼容接口，默认 DeepSeek），
  后台线程防抖 / 缓存 / 超时，`submit` / `poll` 非阻塞。`PredictConfig` 是配置的 `[predict]` 分节。
  只在组句中联想，一次请求给云端词（容错校验后补进候选第一页末尾 `[predict] slots` 格，缺省 2，不预留不占位，前面的本地候选不挪；排布在 Core `CandidateLayout`）和整句补全（preedit 右侧，Tab）；上屏后不联想，本地历史不进请求。
  `CloudGlossFiller` 是释义兜底（Core `GlossFiller` trait，与 Predictor 分开的线程与通道，攒 1.5 秒 / 8 个词发一次，问过不再问）：随包释义表没有的词库词 / 云端词上屏后入队，
  结果壳每秒 `Engine::poll_glosses` 经 `Translator::learn` 写进 `qingjian-translate::PersonalGlossary`（`user-glossary-<语言>.tsv`，`LayeredTranslator` 个人表优先）；随云联想开关一起开。
  `?` 开头是问字模式（`PredictionKind::Question`，答案带读音、不校验拼音）；`PredictionKind::Translate` 是壳里快捷键触发的「翻译选中文字」（双向：汉字为主译成学习语言，外文译回中文，`prediction::translation_target`），译文走结果的 `sentence`。
- `crates/qingjian-format`：`.qj` 数据容器（`Container` mmap 读、`Writer` 写、`Table<T>` / `Text` 零拷贝视图、`hash` 可落盘哈希索引、`Metadata` 名称 / 许可证 / 署名）。
  词库与语言模型都能 `write_qj` / 从 `.qj` 打开，启动 50 ms；`cargo run --release -p qingjian-dict-convert -- pack dict|lm --name … --license …` 生成 `data/generated/{dict,lm}.qj`，
  `bundle.sh` 在 TSV 更新时自动重打并只把 `.qj` 打进包。设计见 `docs/design/architecture.md`「数据文件：`.qj` 容器」。
- `crates/qingjian-neural`：`CharScorer`，Core `sentence::SentenceScorer` trait 的实现：candle 加载字级 Transformer（GPT-2 风格 decoder，训练仓库（本地 `../train`，私有，不在本仓库）导出的
  `model.safetensors` + `config.json` + `vocab.json`，训练脚本不在仓库里），给「前文 + 整句」按字累加 log 概率；前文的每层 K / V 缓存（`PrefixCache`），
  同一段前文只算一次，每个候选只算自己那几个字（64 字前文 × 8 条 28 ms，Metal）。features `accelerate` / `metal` 换后端，壳用 `metal`。
  Engine 侧在 `engine/rescoring/`：接了打分器就取 Viterbi 前 `RESCORE_PATHS` = 6 条路径按 `路径分 + λ·(神经分 − 静态二元分)` 重排（λ `NEURAL_WEIGHT` 0.5，
  个人 n-gram / 用户加分 / 代价不动），分走「前文 + 文本 → 神经分」缓存 `NeuralCache`；同步打分器（`with_sentence_scorer`，CLI 评测）当场补分，
  异步的（`with_async_sentence_scorer`，后台线程 `RescoreWorker`）查询不等模型：缺分的记下来，壳停键后 `request_rescoring`、`poll_rescoring` 到了再 `query` 一次。
  前文优先用壳给的应用光标前文（`set_rescoring_context`），没有用本会话最近 64 个上屏字符。CLI `--neural <导出目录>`（`--neural-weight` / `--neural-context` / `--neural-async`）。
- `crates/qingjian-lm`：`BigramModel`，Core `sentence::LanguageModel` trait 的实现，从 `data/generated/lm.qj`（或 `lm-unigram.tsv` / `lm-bigram.tsv`）加载
  （没有这两个文件就退化为一元词频整句）。数据由 `tools/corpus/parquet_to_text.py`（uv 脚本，HF parquet → 简体纯文本）加
  `cargo run --release -p qingjian-dict-convert -- bigram data/corpus/*.txt` 生成；语料在 `data/corpus/`（gitignore）。
- `crates/qingjian-platform`：`Config`（TOML 配置文件，`[general]` / `[shortcut]` / `[fuzzy]` / `[dictionaries]` / `[apps]` / `[predict]` 分节，首次运行写模板，
  `set_value` 用 toml_edit 原地改键保留注释；`[model] enabled` 本地整句模型开关，`LocalModelConfig`）；`extra_dictionaries` 列出 / 加载随包领域词库与用户 `dicts/`（mac 壳与 Windows Server 共用，同名 `.qj` 优先于 `.tsv`）；`protocol` 模块是 Windows Server ↔ TSF DLL 的 IPC 协议类型（`ClientMessage` / `ServerMessage` / `Frame` / `PreeditSegment`，全 serde，两端共用，见 `docs/design/architecture.md`「Windows：TSF」）。
- `apps/cli`：测试工具，`cargo run -p qingjian-cli -- kaifa`；`--predict` 强制开云联想并等结果打印，交互模式下上屏后也联想；
  `--typing` 逐键计时（性能测试用 release 构建跑，目标每键 10 ms 以内）；`--replay <input-log.jsonl>` 回放评测：把日志里每次上屏的键重新喂给引擎，
  按来源算首选 / 前五命中率、平均名次、不在候选的条数，打印没命中的例子（`--misses N`）；只在内存里学习不写文件，加 `--user-dict` 可带上现有学习数据。
  `--eval-text <文本>...` 整句评测：把用户自己写的中文文本按标点切句、按词库读音转成全拼，冷启动喂给引擎看整句能不能还原原句
  （首选命中率 / 字准确率 / 查询耗时；不依赖日志里当时选了什么，给整句排序与语言模型的改动当尺子），`--eval-save` 冻结成
  `句子\t拼音\t上文` 三列文件，之后直接 `--eval-text` 它保证比的是同一份句子（本机的在 `data/eval/sentences.tsv`）。
  排序、整句、纠错的改动先跑它们再合。
- `apps/macos`：IMK 输入法，已接 Core，自绘候选窗口；源码按 `app / host / imk / candidates / menubar / preferences` 分目录。
  输入法菜单（状态项 + 系统输入源菜单）与偏好设置窗口都是配置文件的前端：只写 `config.toml`，`Host::apply_config` 一条通路热加载，
  激活期间每秒看一次文件 mtime。`apps/macos/scripts/bundle.sh --install` 打包安装到 `~/Library/Input Methods/`（开发用），`--pkg` 做分发用的 pkg（装 `/Library/Input Methods/`，postinstall 跑 `qingjian-macos --register` 注册、启用并切成当前输入源；签名 / 公证靠 `QINGJIAN_SIGN_IDENTITY` / `QINGJIAN_INSTALLER_IDENTITY` / `QINGJIAN_NOTARY_PROFILE`，没设就 ad-hoc；`QINGJIAN_TARGET` 指定架构，成品 `target/pkg/Qingjian-<版本>-<arm64|x86_64>.pkg`）；`scripts/uninstall.sh` 卸载，
  日志在 `~/Library/Logs/Qingjian/`，用户词频在 `~/Library/Application Support/Qingjian/user.tsv`，
  配置在同目录 `config.toml`（云联想 `[predict]`，偏好设置「云服务」页有「测试连接」按钮：`qingjian_predict::ConnectionTest` 起线程发一条最小请求，`Host` 用独立定时器 `CloudTestMonitor` 轮询结果显示到窗口底部；模糊音 `[fuzzy]` 默认都关；`[general]` 学习语言 / 每页候选数 / 翻页键 / 外观 / 竖排横排 / 拼音显示位置 / 英文模式候选开关 / 双拼方案 `shuangpin`（小鹤 / 自然码 / 微软 / 搜狗，空为全拼）/ 日志级别 `log_level`（缺省 info 不含敲的内容，debug 逐键记，热切换）/ 输入日志 `input_log`；`[shortcut]` 模式键 v / u、上屏第一 / 第二个译词的修饰键 `translation` / `translation_second`、删候选 `delete_candidate`（缺省 shift，用户词整删、词库词清学习）、翻译选中文字 `translate_selection`；
  `[apps] english_candidates_off` 按 bundle identifier 列出英文模式不给候选的应用（缺省终端 / 编辑器 / IDE，`*` 前缀匹配）；
  `[dictionaries] domains` 打开随包的领域词库（`Resources/dicts/` 11 本，缺省只开 `idioms`），`disabled` 关掉用户目录 `dicts/` 里的某本导入词库；偏好设置「词库」页随包的可开关、导入的可开关 / 移除，可导入 TSV / Rime yaml / .qj）。输入法进程由 launchd 拉起，看不到 shell 的环境变量：
  密钥写进配置同目录的 `.env`（`QINGJIAN_API_KEY=...`，输入法启动时 dotenvy 读入）或 `config.toml` 的 `api_key`；
  `reasoning_effort` 缺省 `none`（DeepSeek V4 默认思考，不关正文为空）。日志按天分文件留 7 天，删了会重建。
  本地整句模型：`bundle.sh` 把 `data/model/`（或 `QINGJIAN_MODEL_DIR`）三件套打进 `Resources/model/`，用户目录 `model/` 优先；
  `host/model.rs` 在后台线程加载并预热（首次 Metal 编译）后 `set_async_sentence_scorer` 接上，`refresh` 每键先读应用光标前 64 字给 Engine 当前文、查询后
  `schedule_rescoring`，`RescoreMonitor` 停键 80 ms 请求、20 ms 轮询，结果到了重查一次只重画当前页（翻过页 / 动过高亮不动）；「云服务」页有开关（`[model] enabled`）。
  端到端验证可用 `osascript` 的 System Events 往 TextEdit 发按键再读回文本（终端需要辅助功能权限；输入法得在中文模式）。
- `assets/sample/`：手写样例词库与释义表，不是产品数据。`assets/emoji/emoji-zh.tsv` / `emoji-en.tsv` 是 Unicode CLDR 中文 / 英文 annotations 转出的 emoji 表
  （Unicode License v3，可发布；中文词与英文词各配 emoji，两张表加载时合成一张），
  `cargo run --release -p qingjian-dict-convert -- --out-dir assets/emoji emoji --language zh data/cldr/annotations-zh.json data/cldr/annotationsDerived-zh.json`（en 同理）。
  英文词表词频：`uv run tools/corpus/english_frequency.py data/generated/english.tsv -o data/generated/english-frequency.tsv`，再 `... english <词表> --frequency <那个文件>`。
- 提交前检查：`.githooks/pre-commit`（fmt + clippy，`git config core.hooksPath .githooks` 启用一次）；测试在 CI 与每批改动里跑。
- 发版：推 `v<版本>` 标签触发 `.github/workflows/release.yml`（macOS runner 打两个架构的 pkg、建 Release、生成官网用的 `releases.json`），`ci.yml` 在 Linux 上跑 fmt / clippy / test（排除 `qingjian-macos`）；
  更新日志手写在 `CHANGELOG.md`（`## 版本 · 日期 · 渠道` 一节一版，渠道 alpha / beta / rc / stable），产品数据由 `tools/release/data-bundle.sh` 传到 `data` 预发布 Release 供 CI 下载，流程见 `docs/notes/release.md`。
- `tools/gloss-gen`：用 LLM 批量生成释义表：`cargo run --release -p qingjian-gloss-gen -- generate`（密钥读 `QINGJIAN_API_KEY`，
  结果 JSONL 在 `data/generated/`，不进 git、可续跑，`--limit 80` 试跑）再 `... export`（写 `glossary-{en,ja}.tsv`，产品数据在 `assets/glossary/`，见那里的 README；
  格式 `词\t词性. 译词[|假名]`）。CLI 与 bundle.sh 用的就是这两个文件。
- `tools/dict-convert`：产品数据的生成工具，输出到 `data/generated/`（gitignore）。`lexicon` 从 `assets/lexicon/`（自建词库源：规范字 + 常用词 + THUOCL 领域词）
  加 Unihan 读音（`data/unihan/Unihan_Readings.txt`）、LLM 多音字标注（`gloss-gen pinyin`，结果 `data/generated/pinyin-llm.jsonl`，不进 git）、语料词频（`lm-unigram.tsv`）
  建基础词库 `dict.tsv`（8.7 万条）并把 THUOCL 领域词按语料次数 < 50 拆成 `dicts/<领域>.tsv` + `.qj`（11 本、13 万条，`--domain-keep-min`），流程见 `assets/lexicon/QINGJIAN.md`；`english` 转 `assets/lexicon/05_english/00_all_words.tsv`；`cedict` 是释义表备用来源；
  `bigram` 统计语料；`mine` 从语料挖词库没收的高频词并过滤（`oov_filter.rs`：虚词规则 + 相邻字对 PMI≥3，`--candidates` 只重过滤；`lexicon --extra-words` 并入）；`pack dict|lm|glossary` 打 `.qj`（释义表也进容器）。雾凇拼音（GPL）已彻底移除，不要再引入。

`docs/` 分四类（索引在 `docs/README.md`）：
- `design/` 是设计来源：`architecture.md`（架构约束、crate 划分、各平台技术决定、`.qj` 容器）、`candidate-ui.md`（候选窗口与按键约定）、
  `landscape.md`（水杉 / Rime、数据源与许可）。
- `plan/`：`roadmap.md`（阶段与已完成项）、`todo.md`（待办清单，按优先级）。
- `notes/`：工程记录，`performance.md`（历次性能优化的起因、定位与改法，性能改动做完要在这里记一节），以后的复盘也放这里。
- `user/`：**用户文档**，官网（独立仓库 `qingjian-web`）构建时拉取、按目录结构渲染成「文档」页。一个文件夹一个分组（`index.md` 给分组标题与顺序），
  一个 `.md` 一页（frontmatter `title` / `order` / `description`），文件名 ASCII，图片同目录；措辞面向用户，不出现实现词。约定全文见 `docs/user/README.md`。

当实现与规划产生分歧时，以实际代码为准，并同步更新文档，不要让两者长期漂移。
技术方向的改动写进 `docs/design/`，计划改动写 `docs/plan/`，性能 / 复盘一类的工程记录写 `docs/notes/`；
**用户能感知的行为改了（按键、菜单、偏好设置、数据文件），同一个提交里改 `docs/user/` 对应的页**；不要往 README 里加技术内容。

## 常用命令

```bash
cargo build                              # 构建整个 workspace
cargo test                               # 全部测试
cargo test -p qingjian-core              # 单个 crate
cargo test -p qingjian-core <test_name>  # 单个测试（按名字过滤）
cargo test -- --nocapture                # 保留 println! 输出
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

`cargo run -p qingjian-cli -- <拼音>...` 是 Core 的主要验证方式。macOS 输入法（Phase 2）无法通过 `cargo run` 验证：IMK 需要打包成 `.app`、装到
`~/Library/Input Methods/`、注销或重启输入法进程才会生效。Core 的验证要靠
`docs/plan/roadmap.md` 中规划的 CLI 测试工具和单元测试，不要依赖跑起真实输入法。

## 架构约束（这些是本项目的核心设计决定，不要违反）

**Core 与平台层严格解耦。** `qingjian-core` 及其兄弟 crate 必须平台无关：
词库、拼音解析、候选生成、排序、用户词频学习、翻译全部属于 Core。
平台层（IMK / TSF / IBus-Fcitx）只做两件事：把系统输入事件翻译成 Core 的输入，
把 Core 返回的候选画到候选窗口。**平台层里不允许出现排序逻辑、词库访问或翻译调用。**
判断标准：换掉 IMK 换成 TSF，不应该需要改 Core 的任何一行。

**规划中的 crate 划分**（`docs/design/architecture.md`）：
`crates/` 下 `qingjian-core`（composition / parser / candidate / ranking）、
`qingjian-dictionary`、`qingjian-translate`、`qingjian-learning`、`qingjian-platform`；
`apps/` 下 `macos` / `windows` / `linux` 三个平台外壳。

**一个候选词只显示一种辅助语言。** 用户配置 Primary Language + 单个 Learning Language。
不要设计成 `translations: Vec<Translation>` 或 `HashMap<Lang, String>` 这类多语言并列的数据结构——
那会在 API 层面把「一次只学一种语言」这条产品原则给破坏掉。翻译是候选词的
annotation（可选、单条），不是并列的第二套候选系统。

**输入优先于学习。** 任何为学习功能增加的延迟、弹窗、UI 干扰都是设计错误。
翻译查询不能阻塞候选生成——Core 必须能在翻译尚未就绪时先返回候选。

**开发顺序。** `docs/plan/roadmap.md` 的阶段是有依赖关系的：Phase 1 Core（含 CLI 测试工具）→
Phase 2 macOS IMK → Phase 3 翻译 → Phase 4 学习 → Phase 5 Windows/Linux。
先把 Core 和 CLI 打通再碰平台 API，否则会在没有可测试内核的情况下调试 IMK。

## 约定

- 代码标识符一律英文，注释与文档用中文；`thiserror` 的 `#[error]` 文案用英文，日志与 UI 文案用中文。
- 代码分层：一个 struct / enum / trait 及其 impl 单独一个文件，模块文件只做 `mod` 声明、re-export 与自由函数，不把一个 crate 平铺在 `lib.rs` 里。
  有子模块的模块用 `foo/mod.rs`，**不用** `foo.rs` + `foo/` 并列的写法（维护时容易看混）。
- 文件长度：单文件不超过 800 行，目标 500 行以内；测试超过 200 行就搬到 `tests.rs`（多时 `tests/` 按主题分文件）。
  大类型的 `impl` 按职责拆成子模块，每个文件一个 `impl Foo { … }`（`host/settings.rs` 这样），结构体与构造留在 `mod.rs`，
  跨文件用到的私有方法 / 函数标 `pub(super)`，兄弟模块里的自由函数要显式 `use super::sibling::f`。
  一个职责连带它专用的类型收进一个目录：`engine/commit/mod.rs` 放 `impl Engine` 的上屏部分，`chain.rs` / `last.rs` / `transition.rs` 放只有它用的类型
  （`engine/query/`、`engine/learning/`、`engine/prediction/` 同理）。
- 同一词干的兄弟文件合成一个子模块目录，**绝不用文件名前缀分组**。这条对**共享前缀的一组文件**同样成立，不只是 `foo.rs` + `foo_bar.rs`：
  - `foo.rs` + `foo_bar.rs` → `foo/mod.rs` + `foo/bar.rs`；
  - `key_event.rs` + `key_outcome.rs` → `key/mod.rs` + `key/{event,outcome}.rs`（哪怕没有 `key.rs` 这个共同父文件）；
  - `preedit_kind.rs` + `preedit_segment.rs` → `preedit/mod.rs` + `preedit/{kind,segment}.rs`；
  - 也包括 `committing.rs` + `commit_chain.rs`、`query/` + `querying.rs` 这种。
  判断：两个及以上文件名共享一段前缀且同属一个概念，就该收进以那段前缀命名的目录，前缀落到目录名上、后半段做文件名。
- 结构体 / 枚举字段逐条 `///` 注释，字段之间空一行。
- 依赖：`cargo add`，共用包提到根 `[workspace.dependencies]`；错误用 `thiserror` 不用 `anyhow`；日志用 `tracing` 门面。
- 版本号：`crates/*` 是内部库，用 `version.workspace = true` 跟 workspace 一起走；**`apps/*` 每个壳是各自独立发布的产品，写死自己的 `version`，不跟 workspace 同步**（macOS 修的 bug 不该让 Windows 涨版本号）。
  例：mac 到 `0.1.1`、win 还在 `0.1.0`。发版标签按平台加前缀（`macos-v<版本>` / `windows-v<版本>`），CI 各读各的 app 包版本比对；`bundle.sh` 与 `release.yml` 都读 `apps/<平台>/Cargo.toml`（Windows 是一个产品两个 package：`apps/windows/server`（Server 进程）与 `apps/windows/tsf`（TSF DLL），版本读 `server/Cargo.toml`；不合成一个 crate，因为 DLL 不能带 Engine 的依赖树，见 `apps/windows/README.md`）。
- 交流用中文。
