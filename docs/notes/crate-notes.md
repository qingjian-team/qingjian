# 各 crate 的实现要点

CLAUDE.md 只保留目录地图与规则，每个 crate / app / tool 的实现细节收在这里：入口类型、数据文件、常数、生成命令。
改了实现要同步改这里；与代码冲突时以代码为准。

## crates/qingjian-dictionary

词库（TSV 解析或 `.qj` mmap），键按字节序排好，查询逐音节位置二分收窄（简拼位置按音节块跳扫），
`lookup_pattern`（≥ 模式长度）与 `lookup_exact`（正好等长）同一套实现。词库键以 `v` 表示 ü，
TSV 解析、查询与生成工具把 `lue` / `nue` 统一成 `lve` / `nve`。
旧 `.qj` 含这些键时，加载器建立规范化的内存词库。新 `.qj` 继续使用 mmap。
辅码码表（`aux_code_table/`，`AuxCodeTable` = `Kind::AuxCodeTable` 的 `.qj`）是另一套存储：`TEXT` 词 arena + `CODE` 码 arena +
`ENTR` 条目（12 字节，词字节序升序、同词相邻）+ `HASH` 词 → 条目区间起点（`qingjian_format::hash`）。查询只有
`AuxCodeLookup::code_with_prefix` 一个方法：候选词逐个问「有没有以当前码段开头的码」。导入 `import_aux_code_table`
解析 Rime `.dict.yaml` 的 `columns`（缺省 text / code / weight）与 `import_tables`（相对主文件目录合表），
「有词无码」与非法码进统计返回、不静默跳过；一个可用条目都没有时报 `NoCodeEntries`。

`CodeTable` 是形码码表（五笔），与词库并列的另一类查表：键是编码本身（`ggll`），不是拼音音节序列，
格式 `词\t编码\t词频`（`assets/wubi/wubi86.tsv`，8.9 万条），按 `(编码, 词频降序)` 排好、`lookup` 二分定位前缀区间。
编码打全的词（`exact`）排在同前缀的更长编码词前面，这就是一级 / 二级简码的取法，不需要另做简码表。
与词库的一处关键差别：码表没有 `.qj` 容器，只读 TSV。

## crates/qingjian-core

模块：`composition`（缓冲区与光标；中文模式下 Shift+字母按小写进 `buffer` 参与匹配、大写记在 `shifted`，`typed_text` 还原后用于原样上屏）/ `parser` / `correction`（拼写纠错：整段一处编辑的候选纠正 + `typo` 音节级敲错变体表，后者进整句词图当带代价的边）/
`candidate` / `ranking` / `shortcut` / `sentence` / `fuzzy` / `shuangpin`（双拼：七套方案键位表、键 → 全拼解码与消耗换算）/ `zhuyin`（大千注音：键 → 注音符号 → 拼音，`[general] zhuyin` 开关，声调只判音节完整不进查询）/ `emoji` /
`english`（英文模式候选）/ `engine`（`query::EnglishTail`：句末英文词并入整句，`woxiangxuehaorust` → 我想学好rust，尾段也像拼音时按分数与拼音读法比）。
辅码（`engine/aux_code.rs`）：`Engine.aux_code: Option<String>` 是码段（`None` 拼音态，`Some("")` 刚触发或删空停在辅码态——`Engine.aux_keep_empty`，配置 `[general] aux_code_keep_empty` 缺省开），
不进 `Composition`；`aux_trigger`（配的触发键 + 光标在段尾 + 作用域能完整切分 + 双拼韵母键优先）、`enter_aux`、
`push_aux_code`（只收 a-z）、`clear_aux`；退格在辅码态内部分派（删码；删空按 `aux_keep_empty` 停在辅码态或回拼音态，空码段再退格退出），`commit_with` / `take_raw` /
`punctuate` / `clear` 都收尾清码。查询在 `rank` 之后**反向过滤**：候选词逐个 `AuxCodeLookup::code_with_prefix`
（码表在 `Engine.aux_codes`，`set_aux_codes` 注入），不命中的隐藏，命中的按「完全匹配码 > 码长降序 > 原词频序」
重排（stable sort 保住原序），命中码（没在筛码时是词的首条码）写进 `Candidate.aux_code`；码段非空时跳过整句 / 英文 / 快捷 / emoji / 自定义短语
与云联想。preedit 分段多出 [触发键 `Typed`][码段 `MarkedKind::AuxCode`]，见 `Query::marked_segments`。

形码（五笔）在 `engine::query::code`。`Engine` 上有两个开关：`set_code_table`（码表）与 `set_phonetic`（拼音侧参不参与），
在 `query_inner` 进切分之前按这两个分派——只有拼音 / 只有形码（`query_code`）/ **两边都开（`query_mixed`，混输）**。
编码按前缀查表，`CandidateKind::Code` 的候选 `syllables` 为空、上屏吃掉整段作用域（`whole_scope`）。
混输是「拼音那条 Query 前面插上形码候选」：拼音读不出来时（`ggll`）整个按形码走，两边都空才算错；
按文本去重，编码打全的形码词在前、拼音居中、只命中前缀的形码词垫后（一律形码在前的话 `kai` 的首选会变成编码 `kaik` 的词）；混输下模式键同双拼换成大写，四码以内不做拼写纠错，且五笔码最长 4 位、第 5 个字母起自然只剩拼音。
拼音那套在纯形码下全部不适用，靠 `modes()` 返回 `ModeKeys::LETTERLESS` 与 `active_correction` 直接返回 `None` 关掉；
译词标注、生词记录、输入日志、用户选择学习与个人 n-gram 仍照常工作。
`Engine` 是对外唯一门面，`Translator` / `Learner` trait 在 `engine` 模块；词库是「主词库 + 附加词库（`set_extra_dictionaries`）+ 用户词」的列表；繁体输出（`traditional` 开关与 `traditional_map` 映射）依赖 `ferrous-opencc`（`s2tw`）在出候选与上屏边界转换，内部保持简体。
- 中英混输的英文词位置：`Engine::set_chinese_first`（配置 `[general] chinese_first`，缺省关）关着时拼音不像话的输入英文排第一（`extras::insert_english`，
  用户老选中文词时仍让中文在前），开着时整句先插、英文词紧随其后排第二（`query_inner` 里两步的先后按开关掉转）；句末英文词并入整句（`EnglishTail`）不受它影响。
  缺省关是回放定的（9241 词 / 269 条英文上屏：缺省开英文首选 82.5% → 7.1%）。
- `custom_phrase::merge_replacements` 把平台给的「输入码 → 短语」表（macOS 系统文本替换）并进配置里的自定义短语：每条占该码最靠前的空位（1–9），
  输入码不是小写字母、已有同码同文本、九位都满的跳过；Core 不管数据从哪来。
- 拼写纠错（`correction`）：整段一处编辑的变体里相邻换位允许末尾音节没敲完（`loose_segmentation`，`mignt` → `ming t…`；
  有「每个音节都完整、末尾不是落单单字母」的变体时残尾变体不参与，免得 `keyyi` 的 可以 被 `ke yi y…` 抢走），
  纠正之间比较时换位减 `TypoCosts::correction_transpose_discount`（1 nat，CLI `--tune correction-transpose=`），与原样比不减；
  2026-09-15 回放 283 词 / 23 句：词首选 89.0%、整句 82.6%，与改前持平（折扣若也用在与原样比，`zhongwne` 会误纠成 中文，整句掉一条）。

`EngineSession` 保存可挂起的组句、标点、历史与学习链，`Engine::swap_session` 在同一个引擎里交换输入状态，共用词库与落盘服务。切换上下文时清除查询及异步预测缓存，并由平台恢复各自私密状态。

`Engine::raw_preedit()`（`engine/raw/`）只读返回 `RawPreedit { text, cursor_bytes }`：完整未上屏组合及 UTF-8 字节光标，与随后 `take_raw()` 共用文本生成，保留大小写、显式分隔符及光标后的剩余内容，不包含待处理辅码；不运行候选查询、不学习、不记日志、统计、历史或展示回报。
注音继续输出符号；光标按解码单元内按键前缀产生的符号数映射到完整单元的同序字符边界，轻声先敲时采用逻辑位置，一声空格不占显示字符。未知键仍按原解码规则聚到尾串，其光标跟随输出位置而可能不单调；首位固定 0，末位固定完整文本长度。`take_raw()` 的提交和清理顺序不变。

`Engine::discard_input` / `EngineSession::discard_input` 用于隐私能力变化时无痕清理输入，包括透传缓冲、学习链和暂存词汇曝光；`set_private` 只切换写入开关，保留已输入的组句。

## crates/qingjian-translate

`Glossary`，本地 TSV 释义表（词性 + 译文）；`LevelTable`，词汇等级表（`assets/levels/levels-{en,ja}.tsv`，CEFR A1–C2 / JLPT N5–N1，
`uv run tools/corpus/levels.py` 从 `data/levels/` 的原始 CSV 生成，来源与许可见 `assets/levels/README.md`），「统计」页按级数词汇用，不进候选。

## crates/qingjian-learning

- `FrequencyLearner`：用户选择次数（`user.tsv`）、按输入串记的选择（`user-choices.tsv`，词级排序里同输入串选过的优先）、用户词（`user-words.tsv`，主词库同格式，
  Engine 与主词库一起查）、个人英文词（`user-english.tsv`，回车原样上屏的英文词与选过的英文候选，与随包英文词表一起出候选且在前）、
  个人敲错表（`user-typos.tsv`，接受过的 (敲的, 要的) 音节对，词图敲错边与整段纠错的代价按它打折）与个人 n-gram（`user-ngram.tsv`，Core `sentence::UserNgram`，
  二元 + 三元在线计数，整句转换与词级排序里与静态模型插值；Tab 接受的云端整句按 `sentence::segment_text` 切词后也记；
  连着选出的两个词记够次数自动造词进用户词，一段拼音分几次选完的合成词记两次也造）。
- `InputLog`：输入日志（`input-log.jsonl`，每次上屏一行：敲的键、切分、看到的前几个候选、选了第几个、来源、纠错、撤销，
  Core `InputLogger` trait 的落盘实现，`[general] input_log` 缺省开，只写本机，给离线回归评测与个人模型用）。
- `UsageStats`：输入统计（`usage.tsv`，按天记汉字 / 中文词 / 英文词 / 上屏次数，Core `UsageMeter` trait 的实现，Engine 每次上屏 `Usage::of_text` + 按来源定词数，
  整句按 `segment_text` 切词数；与输入日志无关，偏好设置「统计」页显示，`book_scale` 折成几本《某书》）。
- `VocabularyBook`：词汇记录（`user-vocab.tsv`，Core `VocabularyTracker` trait 的实现：学习语言的每条译词看到过几轮 / 上屏过 / ⌥+数字 打出过几次；Core 私密输入统一跳过曝光和提交写入，但仍可读取已有记录用于排序和生词标记；
  Engine `annotate` 据此填 `Sense::fresh`，看到轮次不到 `FRESH_UNTIL` = 3 的译词壳里画橙色；「看到」按上屏那一刻屏幕上那一页算，壳每次画完 `Engine::note_displayed` 告知当前页）。
- 各表落盘走 Core `storage::write_atomic`（临时文件 + fsync + 改名），加载按行容错（坏行警告跳过，真读不了壳退回内存学习），
  壳激活期间每 60 秒 `Engine::flush_learning`；IMK 回调边界 `imk::catch_panic` 拦 panic、缓冲区字母原样上屏（见 architecture.md「崩溃不丢」）。

## crates/qingjian-predict

- `CloudPredictor`：`Predictor` trait 的网络实现（async-openai，OpenAI 兼容接口，默认 DeepSeek），后台线程防抖 / 缓存 / 超时，`submit` / `poll` 非阻塞。
  `PredictConfig` 是配置的 `[predict]` 分节。只在组句中联想，一次请求给云端词（容错校验后补进候选第一页末尾 `[predict] slots` 格，缺省 2，不预留不占位，
  前面的本地候选不挪；排布在 Core `CandidateLayout`）和整句补全（preedit 右侧，Tab）；上屏后不联想，本地历史不进请求。
  简拼（半数以上音节是缩写）的请求 `max_items = 0`，只求整句补全（`prediction::mostly_abbreviated`）：按声母凑出来的词大多是生造词，
  拼音校验又按首字母序列匹配放行缩写，拦不住；问字模式的答案不受这条限制。
- `CloudGlossFiller`：释义兜底（Core `GlossFiller` trait，与 Predictor 分开的线程与通道，攒 1.5 秒 / 8 个词发一次，问过不再问）：
  随包释义表没有的词库词 / 云端词上屏后入队，结果壳每秒 `Engine::poll_glosses` 经 `Translator::learn` 写进 `qingjian-translate::PersonalGlossary`
  （`user-glossary-<语言>.tsv`，`LayeredTranslator` 个人表优先）；随云联想开关一起开。
- 问字键（缺省 `u`）开头是问字模式（`PredictionKind::Question`，答案带读音、不校验拼音），`?` 开头要 `ModeKeys::question_mark` 开着才算（配置 `[shortcut] question_mark`，缺省关，壳用 `Engine::takes_question_mark` 决定空缓冲区的 `?` 是入口还是标点）；`PredictionKind::Translate` 是壳里快捷键触发的「翻译选中文字」
  （双向：汉字为主译成学习语言，外文译回中文，`prediction::translation_target`），译文走结果的 `sentence`。

## crates/qingjian-format

`.qj` 数据容器（`Container` mmap 读、`Writer` 写、`Table<T>` / `Text` 零拷贝视图、`hash` 可落盘哈希索引、`Metadata` 名称 / 许可证 / 署名）。
词库与语言模型都能 `write_qj` / 从 `.qj` 打开，启动 50 ms；`cargo run --release -p qingjian-dict-convert -- pack dict|lm --name … --license …`
生成 `data/generated/{dict,lm}.qj`，`bundle.sh` 在 TSV 更新时自动重打并只把 `.qj` 打进包。设计见 `docs/design/architecture.md`「数据文件：`.qj` 容器」。

## crates/qingjian-neural

`CharScorer`，Core `sentence::SentenceScorer` trait 的实现：candle 加载字级 Transformer（GPT-2 风格 decoder，训练仓库（本地 `../train`，私有，不在本仓库）导出的
`model.safetensors` + `config.json` + `vocab.json`），给「前文 + 整句」按字累加 log 概率；前文的每层 K / V 缓存（`PrefixCache`），
同一段前文只算一次，每个候选只算自己那几个字（64 字前文 × 8 条 28 ms，Metal）。features `accelerate` / `metal` 换后端，壳用 `metal`。

Engine 侧在 `engine/rescoring/`：接了打分器就取 Viterbi 前 `RESCORE_PATHS` = 6 条路径按 `路径分 + λ·(神经分 − 静态二元分)` 重排（λ `NEURAL_WEIGHT` 0.5，
个人 n-gram / 用户加分 / 代价不动），分走「前文 + 文本 → 神经分」缓存 `NeuralCache`；同步打分器（`with_sentence_scorer`，CLI 评测）当场补分，
异步的（`with_async_sentence_scorer`，后台线程 `RescoreWorker`）查询不等模型：缺分的记下来，壳停键后 `request_rescoring`、`poll_rescoring` 到了再 `query` 一次。
前文优先用壳给的应用光标前文（`set_rescoring_context`），没有用本会话最近 64 个上屏字符。CLI `--neural <导出目录>`（`--neural-weight` / `--neural-context` / `--neural-async`）。

## crates/qingjian-lm

`BigramModel`，Core `sentence::LanguageModel` trait 的实现，从 `data/generated/lm.qj`（或 `lm-unigram.tsv` / `lm-bigram.tsv`）加载
（没有这两个文件就退化为一元词频整句）。数据由 `tools/corpus/parquet_to_text.py`（uv 脚本，HF parquet → 简体纯文本）加
`cargo run --release -p qingjian-dict-convert -- bigram --phrases assets/lexicon/phrases.tsv --phrases assets/lexicon/domain_words.tsv --brand assets/lexicon/brand.tsv --brand assets/lexicon/mixed_words.tsv data/corpus/*.txt` 生成；语料在 `data/corpus/`（gitignore）。
短语层不当 token 统计（分词时摘掉、统计完按成分合成一元 / 二元，短语得分等于原来两个词的路径，见 `bigram.rs` 模块注释），品牌词按给定次数写进一元与句首二元。

## crates/qingjian-platform

输入方案是**两条独立的轴**：`Scheme`（拼音侧：全拼 / 双拼七套 / 大千注音 / 关，`[general] scheme`）
与 `[general] wubi`（形码侧：空为关 / `wubi86`）。两边都开就是**混输**（`GeneralConfig::mixed`）。
`scheme_label(pinyin, wubi)` 是状态条显示的方案名（形码在前），做成自由函数而不是存进 `RouterConfig`——
存了会与那两项冗余、手搓配置的地方就漂移（状态条那条测试正是这么发现的）。

演进：2026-09-16 之前是 `shuangpin` + `zhuyin` 两个键表达同一个维度，先并成单选的 `scheme`，
同一天又拆成两条轴（单选的 `scheme` 表达不了混输）。旧键（`shuangpin` / `zhuyin` / `scheme = "wubi86"`）
都还在读、文件不自动改写。


`Config`（TOML 配置文件，`[general]` / `[shortcut]` / `[fuzzy]` / `[dictionaries]` / `[apps]` / `[predict]` 分节，首次运行写模板，
`set_value` 用 toml_edit 原地改键保留注释；`[model] enabled` 本地整句模型开关，`LocalModelConfig`；
中英模式两项：`[shortcut] switch_mode`（`SwitchKey`：shift / control / none，单击切换键）与 `[general] english_mode`（内置英文模式总开关））；
`extra_dictionaries` 列出 / 加载随包领域词库与用户 `dicts/`
（mac 壳与 Windows Server 共用，同名 `.qj` 优先于 `.tsv`）；`code_tables` 同构地列出 / 加载随包根 `codes/` 与用户 `codes/` 的码表
（`[aux_code] disabled` 是黑名单，`[general] aux_code_key` 缺省 `;` 且校验后退回缺省、`aux_code_show` 是显示码开关）；
`protocol` 模块是 Windows Server ↔ TSF DLL 的 IPC 协议类型
（`ClientMessage` / `ServerMessage` / `Frame` / `PreeditSegment`，全 serde，两端共用，见 `docs/design/architecture.md`「Windows：TSF」；
`PROTOCOL_VERSION` = 6，`PreeditKind::AuxCode` 对应 Core 的 `MarkedKind::AuxCode`，`Frame.aux_code_show` 随帧下发显示码开关）。

## crates/qingjian-render

横排矩阵：`Frame::columns` 不为 0 时 `Layout::Horizontal` 走 `renderer/matrix.rs`（列宽用帧里的 `column_ems`，Core `Grid::column_ems` 按整份候选估、
单格封顶 `MAX_CELL_EMS` = 4 字宽，滚动时窗口不跳；网格下固定一行信息，放不下的截断）；视口与高亮移动在 Core `candidate::layout::Grid`（`GRID_ROWS` = 6），
预览示例里有 `matrix-horizontal` 场景。mac 壳里这套按键由 `[general] horizontal_grid`（缺省关）加横排两个条件一起开（`Host::grid_keys`）。

自绘渲染器：候选窗一帧 + 主题 → 预乘 RGBA 位图，tiny-skia 栅格 + cosmic-text 文字（fontdb 按平台清单只加载几个字体文件、不扫系统），
自己解析 `trak` 字距表、按主题 gamma 加深笔画；cosmic-text 打了 `opsz` 光学字号补丁（qingjian-team/cosmic-text 分支 `qingjian-opsz`，workspace `[patch.crates-io]` 钉 rev）。
`examples/preview.rs` 出 PNG 与真机截图并排比、`--measure` 与 AppKit 对宽度。mac 壳 `candidates/bitmap/` 贴位图，`[general] renderer = "system"` 切回 AppKit 绘制
（过渡期退路，偏好设置「候选窗口」页可选）；`[general] font` 是候选窗字族名（空为系统字体，`bitmap/font_files.rs` 用 CoreText 按字族名找文件只加载那几个，没装就回系统字体；
设置页 `preferences/font_picker/` 是搜索框 + 列表）。设计与验收见 `docs/design/rendering.md`。

## apps/cli

测试工具，`cargo run -p qingjian-cli -- kaifa`。

- `--predict` 强制开云联想并等结果打印，交互模式下上屏后也联想。
- `--wubi <码表>` 用形码码表（`词\t编码\t词频` 的 TSV）替代拼音：按键当编码按前缀查表，候选不带音节，上屏吃掉整段编码。
- `--typing` 逐键计时（性能测试用 release 构建跑，目标每键 10 ms 以内）。
- `--chinese-first` 打开中文优先（`[general] chinese_first = true` 的排法），配合 `--replay` 比两种英文词位置。
- `--replay <input-log.jsonl>` 回放评测：把日志里每次上屏的键重新喂给引擎，按来源算首选 / 前五命中率、平均名次、不在候选的条数，打印没命中的例子（`--misses N`）；
  只在内存里学习不写文件，加 `--user-dict` 可带上现有学习数据。
  **按日志记的方案逐条切换**（`replay/scheme.rs` 的 `SchemeSwitcher`，读每条的 `scheme` 字段）：整份日志通常只有一套方案，
  所以只在方案串变化时才换码表（`Engine::set_code_table` 收所有权，换一次要克隆 8.9 万条）。日志里是形码但没给 `--wubi` 时，
  那部分只计数——拿拼音的读法喂形码的键会算出看着像真的、实则无意义的命中率。
  含触发键的日志行按壳那样逐键重喂（进辅码态、码段进码段），
  并多打一行「辅码选词 N 条，其中同拼音纯输入首选命中 M 条」（学习闭环的尺子）。
- `--aux-table <路径>`（可多次）装辅码码表（`.qj` 或 `词<Tab>码` TSV）；交互模式与查询模式都按壳的方式逐键喂入，
  配的触发键进辅码态、之后的字母进码段，候选行会带上命中的码。
- `--tune 名=值`（逗号分隔）覆盖个人 n-gram 插值与敲错代价的常数扫网格（名字见 `apps/cli/src/tuning.rs`，Core 侧是 `Engine::set_interpolation` / `set_typo_costs`，壳只用缺省值）。
- `--eval-text <文本>...` 装了码表（`--aux-table`）时多打一行码表覆盖率：词频前 10,000 与全库两段命中比例（与导入统计同源）。
- `--eval-text <文本>...` 整句评测：把用户自己写的中文文本按标点切句、按词库读音转成全拼，冷启动喂给引擎看整句能不能还原原句
  （首选命中率 / 字准确率 / 查询耗时；不依赖日志里当时选了什么，给整句排序与语言模型的改动当尺子），`--eval-save` 冻结成 `句子\t拼音\t上文` 三列文件，
  之后直接 `--eval-text` 它保证比的是同一份句子（本机的在 `data/eval/sentences.tsv`）。排序、整句、纠错的改动先跑它们再合。

## apps/macos

IMK 输入法，源码按 `app / host / imk / candidates / menubar / preferences` 分目录。

- 输入法菜单（状态项 + 系统输入源菜单）与偏好设置窗口都是配置文件的前端：只写 `config.toml`，`Host::apply_config` 一条通路热加载，激活期间每秒看一次文件 mtime。
  输入方案（`[general] scheme`）也在这里装配：双拼 / 注音设给引擎，形码额外按 `paths::code_table_path()` 挂码表
  （用户目录 `wubi/wubi86.tsv` 优先，包里 `Resources/wubi/` 兜底；找不到只警告并按拼音跑）。
- `apps/macos/scripts/bundle.sh --install` 打包安装到 `~/Library/Input Methods/`（开发用），`--pkg` 做分发用的 pkg（装 `/Library/Input Methods/`，postinstall 跑 `qingjian-macos --register`
  注册、启用并切成当前输入源；签名 / 公证靠 `QINGJIAN_SIGN_IDENTITY` / `QINGJIAN_INSTALLER_IDENTITY` / `QINGJIAN_NOTARY_PROFILE`，没设就 ad-hoc；`QINGJIAN_TARGET` 指定架构，
  成品 `target/pkg/qingjian-<版本>-macos-<arm64|x86_64>.pkg`）；`scripts/uninstall.sh` 卸载。
- 日志在 `~/Library/Logs/Qingjian/`（按天分文件留 7 天，删了会重建），用户数据与配置在 `~/Library/Application Support/Qingjian/`。
- 配置项：云联想 `[predict]`（偏好设置「云服务」页有「测试连接」按钮：`qingjian_predict::ConnectionTest` 起线程发一条最小请求，`Host` 用独立定时器 `CloudTestMonitor` 轮询结果显示到窗口底部；
  `reasoning_effort` 缺省 `none`，DeepSeek V4 默认思考，不关正文为空）；模糊音 `[fuzzy]` 默认都关；`[general]` 学习语言（`off` 不显示译文）/ 每页候选数 / 翻页键 / 外观 / 竖排横排 / 拼音显示位置 /
  英文模式候选开关 / 中文优先 `chinese_first` / 双拼方案 `shuangpin`（小鹤 / 自然码 / 微软 / 搜狗 / 智能ABC / 小浪 / 首道，空为全拼）/ 日志级别 `log_level`（缺省 info 不含敲的内容，debug 逐键记，热切换）/ 输入日志 `input_log`；
  `[shortcut]` 模式键 v / u、`question_mark`（缺省关，开了空缓冲区敲 `?` 进问字）、上屏第一 / 第二个译词的修饰键 `translation` / `translation_second`、删候选 `delete_candidate`（缺省 shift，用户词整删、词库词清学习）、翻译选中文字 `translate_selection`；
  `[apps] english_candidates_off` 按 bundle identifier 列出英文模式不给候选的应用（缺省终端 / 编辑器 / IDE，`*` 前缀匹配）；
  `[dictionaries] domains` 打开随包的领域词库（`Resources/dicts/` 11 本，缺省只开 `idioms`），`disabled` 关掉用户目录 `dicts/` 里的某本导入词库；
  偏好设置「词库」页随包的可开关、导入的可开关 / 移除，可导入 TSV / Rime yaml / .qj。
- 系统文本替换（系统设置「键盘 → 文本替换」）：`host/config/text_replacements.rs` 从 `NSUserDefaults` 全局域读 `NSUserDictionaryReplacementItems`
  （每条 `{ on, replace, with }`），激活输入法时重读，变了就经 Core `merge_replacements` 并进配置里的自定义短语再 `set_custom_phrases`；
  `[general] system_text_replacements` 开关（缺省开，「自定义短语」页勾选框），内容可能含证件号、地址，日志只记条数。
- 输入法进程由 launchd 拉起，看不到 shell 的环境变量：密钥写进配置同目录的 `.env`（`QINGJIAN_API_KEY=...`，输入法启动时 dotenvy 读入）或 `config.toml` 的 `api_key`。
- 本地整句模型：`bundle.sh` 把 `data/model/`（或 `QINGJIAN_MODEL_DIR`）三件套打进 `Resources/model/`，用户目录 `model/` 优先；`host/model/mod.rs` 在后台线程加载并预热（首次 Metal 编译）后
  `set_async_sentence_scorer` 接上，`refresh` 每键先读应用光标前 64 字给 Engine 当前文、查询后 `schedule_rescoring`，`RescoreMonitor` 停键 80 ms 请求、20 ms 轮询，
  结果到了重查一次只重画当前页（翻过页 / 动过高亮不动）；「云服务」页有开关（`[model] enabled`）。
- 端到端验证可用 `osascript` 的 System Events 往 TextEdit 发按键再读回文本（终端需要辅助功能权限；输入法得在中文模式）。

## apps/windows

一个产品两个 package：`server`（Server 进程：IPC 分派 + Engine + 命名管道 + 自绘候选窗与悬浮状态条）与 `tsf`（TSF 文本服务 DLL，lib 名固定 `qingjian_tsf`），
外加 `settings`（WinUI 3 设置程序，含「辅码」页）与 `installer`（Inno Setup）。
Server 侧辅码接线：`RouterConfig.aux_code_key` / `aux_code_show`（`apply_config` 热加载）、`assembly` 从随包 `codes/` 与用户 `codes/` 装码表
（`[aux_code] disabled` 过滤，`code_tables::load`）、按键在 `dispatch/key/input.rs` 分流（触发键进辅码态、辅码态里字母进码段、
大写（`shift_letter = "compose"`）先清码段回拼音态再进缓冲区、标点先上屏高亮候选再转全角）、候选窗 `ui/candidates/row.rs` 的 `Row.code` 把码用方括号括起来紧跟在候选词后面（不进 annotation），拼音行 `view.rs` 给码段加下划线。
热加载的 `dicts/` 与 `codes/` 目录与启动同款（修过一处传基础目录的错）。不合成一个 crate，因为 DLL 不能带 Engine 的依赖树，见 `apps/windows/README.md`；
协议类型在 `qingjian-platform::protocol`，设计见 `docs/design/architecture.md`「Windows：TSF」。
输入方案由 `[general] scheme` 一处决定，Server 启动与热加载各装配一次；形码的码表用 `dispatch::code::find_code_table` 找
（用户目录 `wubi/wubi86.tsv` 优先，随包 `assets/wubi/wubi86.tsv` 兜底——走 `assets/` 与 emoji / levels 一致，开发布局也对得上），**路径在启动时定下、热加载不重新找**。
选了形码却没有码表文件时只警告并按拼音跑——配置说五笔、引擎还在拼音是静默错位，宁可吵。
中英模式的两项设置（`[shortcut] switch_mode` 切换键：shift / control / ctrl+space / none，`[general] english_mode` 内置英文模式开关）
由 Server 经协议下发给 DLL（`InputSettings`，见下文「按键行为设置」），改完在下一拍（约 320 ms）生效；
`ctrl+space` 走 TSF 保留键登记（`com/key/preserved.rs` 的 `GUID_SWITCH_MODE`），但先读系统热键
`Hot Keys\00000010`（「输入法/非输入法切换」，缺省就是 Ctrl+Space）：被系统占着时不重复登记、交给系统那条路
（它的转换模式变化由 conversion compartment 回调同步成中 / 英），避免两边各切一次互相抵消。四条切换入口都汇到
`service/mode.rs::set_english_mode` 一处拦住；状态条点击在 Server 侧（`dispatch/status/mod.rs`）按同一项拦，
设置界面在 `settings/src/panel/pages/general.rs`。

TSF 原有数字 / OEM 标点 / 空格键码按当前布局用 `ToUnicodeEx` 解析（bit 2 避免改变键盘状态），
仅接受单个非代理项 UTF-16 单元。字母、小键盘和 AltGr 处理不变，不保证组合音符输入。
拼音显示位置（`[general] preedit`）在 Windows 上分两处落地：Server 把它读进 `RouterConfig.preedit` 并随 `Frame.preedit_mode`
下发给 DLL，DLL（`com/service/key_sink.rs`）按 `inline()` 决定要不要放行内拼音，Server（`ui/candidates/render_data.rs::window_preedit`）
按 `in_window()` 决定候选窗口顶部画不画拼音行；`window` 模式没有组句范围，光标矩形改从 `com/edit/anchor.rs::caret_rect`（当前选区）量。
连不上 Server 时 DLL 自己拉起它（`tsf/src/com/service/launch.rs`）：`ShellExecuteW` 起与 DLL 同目录的 `qingjian-server.exe`
（`uiAccess=true` 的 exe 用 `CreateProcess` 报 740），进程内 5 秒冷却 + 跨进程命名互斥体防止砸出一串 Server；
起完清掉重连退避，下一键就试。Server 只在登录时由「启动」文件夹拉起，中途挂了以前只能等下次登录。

词库导入（设置「词库」页）走 `qingjian-dictionary::import` 转成 `.qj`（空词库拒绝），多选批量、成功的从 `[dictionaries] disabled` 摘掉、页面显示每个文件的结果；
Server 每次轮询比对用户 `dicts` 的路径 / mtime / 长度快照，配置没变也重载新增、同名更新与移除；配置解析失败时词库沿用上次有效的开关（#36）。

中英切换键（`[shortcut] switch_mode`）与内置英文模式开关（`[general] english_mode`）得在**按键到达之前**就知道
（单击判定在 `OnTestKeyUp`、是否登记语言栏按钮），但 DLL 跑在每个应用进程里、拿不到 Server 那份配置，
`%APPDATA%\Qingjian` 对 AppContainer 里的商店应用也读不到（那是给输入日志和 `.env` 用的目录，不该加 ACE）。
所以由 **Server 读配置、经协议下发**（`InputSettings`：`OpenSession` 回包带一次，之后每拍 `SyncMode` 跟着走），
DLL 不读文件、不查 mtime。`SessionOpened` 只回过协议版本对得上的 DLL——老的 `open` 是只写不读，
多回一条会被它当成下一次 `Poll` 的应答而报错，那条连接就废了；老 DLL 从 `ModeSync` 那一拍也能拿到同一份（新字段直接忽略）。

## assets

- `assets/sample/`：手写样例词库与释义表，不是产品数据。
- `assets/wubi/wubi86.tsv`：五笔码表（`dict-convert wubi` 生成，来源与许可见同目录 README，Apache-2.0）。
  `bundle.sh` 拷到 `Resources/wubi/`，Windows 安装器拷到 `{app}\assets\wubi\`（与两边 Server 的找法对齐：macOS `paths::code_table_path`、Windows `dispatch::code::find_code_table`）。
- `assets/emoji/emoji-zh.tsv` / `emoji-en.tsv`：Unicode CLDR 中文 / 英文 annotations 转出的 emoji 表（Unicode License v3，可发布；中文词与英文词各配 emoji，两张表加载时合成一张），
  `cargo run --release -p qingjian-dict-convert -- --out-dir assets/emoji emoji --language zh data/cldr/annotations-zh.json data/cldr/annotationsDerived-zh.json`（en 同理）。
- 英文词表词频：`uv run tools/corpus/english_frequency.py data/generated/english.tsv -o data/generated/english-frequency.tsv`，再 `... english <词表> --frequency <那个文件>`。

## tools/gloss-gen

用 LLM 批量生成释义表：`cargo run --release -p qingjian-gloss-gen -- generate`（密钥读 `QINGJIAN_API_KEY`，结果 JSONL 在 `data/generated/`，不进 git、可续跑，`--limit 80` 试跑）
再 `... export`（写 `glossary-{en,ja}.tsv`，产品数据在 `assets/glossary/`，见那里的 README；格式 `词\t词性. 译词[|假名]`）。CLI 与 bundle.sh 用的就是这两个文件。

## tools/dict-convert

产品数据的生成工具，输出到 `data/generated/`（gitignore）。

- `lexicon`：从 `assets/lexicon/`（自建词库源：规范字 + 常用词 + THUOCL 领域词）加 Unihan 读音（`data/unihan/Unihan_Readings.txt`）、LLM 多音字标注（`gloss-gen pinyin`，
  结果 `data/generated/pinyin-llm.jsonl`，不进 git）、语料词频（`lm-unigram.tsv`）建基础词库 `dict.tsv`（8.7 万条），并把 THUOCL 领域词按语料次数 < 50 拆成
  `dicts/<领域>.tsv` + `.qj`（11 本、13 万条，`--domain-keep-min`），流程见 `assets/lexicon/QINGJIAN.md`；`--extra-words` 并入人工挑的领域词 `assets/lexicon/domain_words.tsv`。
- `english`：转 `assets/lexicon/05_english/00_all_words.tsv`；同编码优先保留含大写的专名写法（Windows ≠ windows），
  展示写法补充表 `07_display_forms.tsv` 后置读入；`cedict`：释义表备用来源。中英混杂词源在 `assets/lexicon/mixed_words.tsv`（`lexicon --extra-words`）。
- `wubi`：Rime 形码码表（`.dict.yaml`，极点 86 五笔）→ `词\t编码\t词频`（`wubi.rs`，`--name` 决定文件名，缺省 `wubi86.tsv`）。
  **码表自带的权重不用**——那是码表顺序不是语料词频，用了同一个词在形码下和在拼音下会排得不一样；词频从青简词库按**词面**交叉回填，
  词库里没有的词给 `UNKNOWN_FREQUENCY = 1`。解析复用 `qingjian_dictionary::import::rime`（与用户导入 Rime 词库同一个解析器，
  形码码表的第二列是编码，格式一样）。词库还没收的词不收（码表整张进，不做取码推导，见 `docs/plan/wubi.md`）。
- `bigram`：统计语料；`--phrases` 给短语层、`--brand` 给品牌词（`assets/lexicon/brand.tsv`，青简 210）与中英混杂词（`mixed_words.tsv`，C盘 / B站：合成计数要成分词在语料里，C 不是 token，只能直接给一元，次数对着同音竞争词定），领域词也走合成计数（语料里只有几十次的词当 token 统计会吸走成分词的二元证据）。
- `mine`：从语料挖词库没收的高频词并过滤（`oov_filter.rs`：虚词规则 + 相邻字对 PMI≥3，`--candidates` 只重过滤）。
- `phrases`：挖短语层（两遍扫语料：相邻两词、两段二元都够频的相邻三词，总次数与对话语料次数都 ≥ 2000 + 边界规则，读音由成分词拼出；我的 / 不知道 / 有没有 这类常用词表不收的组合，
  `assets/lexicon/phrases.tsv`；词库已并入过短语时重跑加 `--refresh`）。
- `pack dict|lm|glossary|codes`：打 `.qj`（释义表也进容器；`codes` 是唯一带计算的一种，见下）。
- `stroke`：CNS11643 全字庫筆順（`data/cns/`，官方 Properties.zip / MapingTables.zip 解出，gitignore）+ 大陆序覆盖表
  `assets/stroke/prc-rules.tsv` → `data/generated/codes/stroke.tsv`（随包笔画表的源数据：7,991 字、127 KB，
  1 横 2 竖 3 撇 5 折 n 点捺；首笔按《通用规范汉字笔顺规范》GF 0023—2020 全对：门字头 / 戶→户 两条前缀规则 + 66 行整字覆盖，阝第二笔随规范改竖）；`--verify` 双对照——笔画数按一级字每 12 字取 1（291 字）、首笔按一级字 3,500 全量，白名单
  `assets/stroke/residual-whitelist.tsv`（7 字）与首笔 `assets/stroke/residual-first-strokes.tsv`（408 字，对照源几何假阳性；首笔不按几何近似放行，不符的字都要逐字进白名单）各自口径之外一处不符即退出码非 0；两张对照表（笔画数 / 首笔几何类别）由 `mmh-reference` 子命令从 hanzi-writer-data 生成到 `data/mmh/`（Arphic 许可，不进仓库、不随包；缺席时跳过对照并提示）。
  来源、许可与验收记录见 `assets/stroke/README.md`。
- `pack codes`：把 `codes/stroke.tsv` 与词库（缺省 `data/generated/dict.qj`）算成随包原生码表 `data/generated/codes/stroke.qj`
  （键位 1→h 横 / 2→s 竖 / 3→p 撇 / 5→z 折 / 点捺 n→n；单字「前 4 笔 + 末笔」，不足 5 笔按实际取；词组每字首笔，
  二字 2 码、三字 3 码、四字以上 = 前三字首笔 + 末字首笔；词里有一个字不在笔画表里整词跳过）；
  `--stroke` / `--dict` / `--output` 改路径，元数据缺省「笔画」/ `OFL-1.1` / CNS11643 数位发展部署名（可覆盖），数据版本取笔画表日期。
  2026-09-20 实测：词库 92,821 词条 / 91,904 词，有码 91,773、无码跳过 131，码表 91,773 条 / 3.0 MB。

## apps/linux

`qingjian-linux-server` 为独立产品 `0.1.0-dev`，只装配本地 Engine、词库、释义、频率学习、个人 n-gram、词汇记录与可选输入日志。
不接云服务或神经重排。`dispatch/session` 交换每个上下文的 EngineSession；真正的能力变化丢弃输入，普通焦点切换隔离保存。
默认面板插件仅转换事件，Shift 模式、候选点击、分页和失焦提交都由 Server 决定。

Unix socket 用共享长度前缀与 Frame（当前公共版本 6）；插件复用一条连接，每个上下文独立会话。Linux v3 扩展逐会话握手、确认 Sensitive/Password/Disable 后接受按下/释放、焦点和点击事实。
候选回报绑定连接代次、上下文和服务端帧序号，仅当前聚焦页的有效释义进入 `note_displayed`，不把生成帧算作已展示。
`[general] preedit` 使用已有 `both` / `inline` / `window`；没有新增 Linux 自绘配置。详见 [linux-fcitx5.md](linux-fcitx5.md)。
