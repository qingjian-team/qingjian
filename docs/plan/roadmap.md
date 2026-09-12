# Roadmap

各阶段有依赖关系：先把 Core 和 CLI 打通再碰平台 API，否则会在没有可测试内核的情况下调试 IMK。

## Phase 1 — Core

- [x] 基础 Composition 状态机
- [x] Candidate 数据模型（含词性 + 译文的 annotation 结构）
- [x] 拼音解析（全拼、`'` 分隔、末尾残缺音节）
- [x] 中文候选生成（词级）
- [x] 基础词库（TSV 内存词库，`assets/sample/` 为手写样例）
- [x] 候选排序（词级规则排序）
- [x] CLI 测试工具（`apps/cli`：候选 + 词性译文 + 各阶段耗时，序号上屏记入用户词频）
- [x] 音节表查找改为 trie
- [x] 简拼（声母缩写 `kf` → 开发，与全拼混用，marked text 自动补 `'`）
- [x] 开发测试词库：曾用雾凇拼音（GPL，不可发布）；2026-09-05 起换成自建词库，见 Phase 2 末尾「产品词库」
- [x] 中→英释义：`tools/dict-convert` 从 CC-CEDICT 生成 11.5 万条（CC BY-SA 4.0，可发布）
- [x] 带词性的中英 / 中日释义表：`tools/gloss-gen` 用 LLM 批量生成（2026-09-05 起覆盖整个自建词库 23.9 万词，词性 + 英文译词 + 日文译词与假名，`assets/glossary/`），
  `Sense.reading` + 汉字注平假名（`candidate::furigana`）；取代 CC-CEDICT 表做随包数据
- [x] Option+数字上屏候选的译文（`Engine::commit_translation`，控制器在 `handleEvent:client:` 里按键码认，其余交父类）
- [x] 整段切不动时取能切分的最长前缀出候选，剩余字母作为未切分尾部保留（`kaifv` → `kai'f'v`）
- [x] 整句转换（bigram + Viterbi），含简拼整句（`wxqcf` → 我想去蹭饭 / `jttqhh` → 今天天气很好）
- [x] 模糊音（Core `fuzzy`，配置 `[fuzzy]`）：z/zh、c/ch、s/sh、n/l、f/h、l/r、an/ang、en/eng、in/ing，
  词库按「每个位置多种写法」一次查出，模糊命中词频减半排序，整句转换同样生效
- [x] 偏好设置「关于」页（2026-09-05）：版本 + 构建号、测试版许可（仓库 `LICENSE` 由 MIT 改为测试版保留所有权利，开源时再定）、数据署名、隐私说明、
  「打开日志目录」「复制诊断信息」；日志缺省 info、`[general] log_level` 热切换
- [x] 双拼（2026-09-05，Core `shuangpin`）：小鹤 / 自然码 / 微软 / 搜狗四套方案，键 → 全拼解码后复用全拼的切分与整句，配置 `[general] shuangpin`，
  偏好设置「通用」页弹出菜单，CLI `--shuangpin`
- [x] 查词性能：`lookup_pattern` / `lookup_exact` 逐级前缀二分收窄（简拼位置按音节块跳扫，小区间线性），
  同一次查询内前缀模式记忆化，排序键预计算 + 只选前 500 条。长输入 20 ms → 0.2 ms，全简拼 `zhgdoima` 70 ms → 1 到 3 ms，单字母 30 ms → 3 到 4 ms
- [x] 逐键性能（2026-09-05，`qingjian-cli --typing` 按前缀逐键计时）：纠错变体先用无分配的「能否完整切分」过滤
  （`zhzhzh…` 16 键 parse 39 ms → 0.2 ms）；整句格子候选跨按键缓存（`sentence::SpanCache`，上屏 / 学习时清，
  `sss…` 15 键 rank 12 ms → 1 ms）；bigram 表改成按前词分组的 CSR 布局；词频 / 语言模型 / 个人 n-gram 的哈希表换 foldhash；
  预选键打包成 u128；没开模糊音不逐条对模糊命中。结果：全拼、简拼长句每键 0.3 到 3 ms，单字母首键 5 ms（模糊音全开 8 ms），
  英文模式每键 0.3 ms 以内；启动 0.9 s（词库 0.24 s + LM 表 0.62 s）
- [x] 数据二进制化（2026-09-05，`crates/qingjian-format`）：`.qj` 容器 = 头 + 分节表 + `META`（名称 / 许可证 / 署名）+ 原样落盘的内存数组，
  mmap 零拷贝打开；词库与语言模型都进了容器（`dict-convert pack dict|lm`），启动 0.93 s → 0.05 s（词库 12 ms + LM 9 ms）
- [x] 产品词库（2026-09-05，见 Phase 2 末尾）

以上「已完成」指 2026-09-06 的状态：自用能用，词库已是自建的产品数据（基础 + 11 本随包领域词库），学习数据原子写、坏文件容忍（见 Phase 2「崩溃不丢」）。

## Phase 2 — macOS

- [x] IMK 空壳（打包、安装、注册、按键到达链路）
- [x] Input Method Kit 接入 Core（`apps/macos/src/host.rs` 进程级单例 Engine）
- [x] 输入事件处理（字母 / `'` / 数字选词 / 空格 / 回车 / 退格 / Delete / Esc / 上下键选词 / 左右键与 Cmd+左右移动拼音光标）
- [x] Composition 显示（marked text 显示原始拼音）
- [x] 自定义竖排候选窗口（`candidate_window.rs` + `candidate_view.rs` 自绘，`theme.rs` 集中可视参数）
- [x] 中文输入
- [x] 候选选择与上屏（上屏后剩余拼音继续组句）
- [x] 候选翻页（PageUp、PageDown / Tab、Shift+Tab / `[` `]`（缺省）或 `,` `.`（配置）翻页，上下键到页边自动翻页，右下角页码；组句期间未识别的编辑动作一律吞掉）
- [x] 拼音光标编辑：左右键移动光标，插入 / 退格 / Delete 相对光标；marked text 里显示真实光标
- [x] 光标作用域：光标停在中间时候选、云端词校验、联想请求都只看光标前的拼音（`Composition::scope`），上屏吃掉作用域后光标落到末尾
- [x] 整句转换（Core `sentence`）：词图 + Viterbi + 束搜索，`Dictionary::lookup_exact` 精确长度查词图格子；整句排第一，`CandidateKind::Sentence`
- [x] 简拼整句：简拼位置按前缀取词进格子（格子多留到 20 个），语言模型在路径上挑读音；`jttqhh` → 今天天气很好、`wjdzjsg` → 我觉得自己是个；
  两字母简拼仍是词优先（`sj` 时间 / `zg` 这个，Viterbi 单词路径分高就不出句子）；有音节没转成字的不算句子；英文词 / 英文补全留在句子前面
- [x] bigram 语言模型（`qingjian-lm`，`LanguageModel` trait 注入）：`dict-convert bigram` 从中文维基 + LCCC 统计，`pack lm` 打成 `lm.qj` 随 bundle 打包，没有时退化为一元
- [x] 个人 n-gram（Core `sentence::UserNgram`，`Learner::user_ngram()`）：上屏词序列在线计数，与静态模型插值进 Viterbi；`user-ngram.tsv` 落盘；
  2026-09-06 加三元（绝对折扣回退到二元，Viterbi 前二词取前驱回指，`CommitChain` 记两个词）
- [x] 自动造词：连着选出的两个词合起来词库没有、记够次数就成用户词（同段拼音两次 / 分段三次）
- [x] 快捷候选（Core `shortcut`）：`rq` / `sj` / `xq` 出日期 / 时间 / 星期，`v` 开头表达式模式出四则运算结果与中文数字（见 candidate-ui.md）
- [x] 中文标点（Core `punctuation`：全角映射、引号配对、数字后的点保持半角；组句期间 `,` `.` 把高亮候选上屏再补标点；配成翻页键时只翻页）
- [x] 候选框顶部自绘 preedit 行与光标（不依赖应用画插入点）
- [x] bundle id 改为 `app.qingjian.inputmethod`（2026-09-06，域名 qingjian.app 注册后；`TISInputSourceID` 与连接名同步，旧 id `com.yenharvey.qingjian` 的输入源要在系统设置里删掉重加）
- [x] 中英切换：Caps Lock 亮着 = 英文模式（默认小写、Shift 大写、标点半角）；按住 Shift 的大写字母直接透传；切换输入源时强制收窗
- [x] 英文模式候选（Core `english`，`Engine::set_english_mode`）：词表精确词 / 前缀补全 / 一处编辑纠正，Tab 与上下键选词（上下键动过之后空格也选），空格回车标点原样上屏；
  `[general] english_candidates` 可关；CLI `--english-mode`
- [x] 按应用关英文候选（2026-09-05）：`[apps] english_candidates_off`，按 `bundleIdentifier` 认，缺省终端 / 编辑器 / IDE 名单，`*` 前缀匹配；
  偏好设置「通用」页勾选框；`[apps]` 分节留给以后的按应用 preedit 模式
- [x] 个人英文词表（2026-09-05，`user-english.tsv`）：回车 / 英文模式直通原样上屏的英文词（切不成完整拼音的字母串）与选过的英文候选都记，
  与随包词表一起出英文候选且在前；修的是 `gist` 这类随包词表里没有的词永远出不了候选、回车多少次也学不会的问题
- [x] 中英混输：整段输入在英文词表里即出英文候选，作为拼音「不像话」时排第一，否则排第二（词表 2026-09-05 起来自 `assets/lexicon/05_english`，ESDB / CSpell，MIT）
- [x] 英文补全：拼音不像话且 ≥ 3 个字母时出该前缀下最常用的 3 个英文词（词频 wordfreq），第一个字母就切不动的输入也出
- [x] 英文直输段：组句中敲 `-` 后整段原样上屏（`no-way`），`-` 不再翻页
- [x] emoji 候选（Core `emoji`，Unicode CLDR 中文 annotations，`assets/emoji/`）：紧跟对应词，右侧标注词
- [x] 密钥：输入法进程读配置同目录的 `.env`（launchd 看不到 shell 环境变量）
- [x] 学习语言、每页候选数、翻页键、外观、模式键从配置文件读取（`[general]` / `[shortcut]`），保存后自动热加载
- [x] 应用图标与输入法菜单图标（`assets/icon/logo.png` → bundle.sh 生成 icns 与多分辨率 tiff）
- [x] 菜单栏「中 / 英」状态项（NSStatusItem，激活时显示，定时轮询 Caps Lock）
- [x] 输入法菜单（状态项 + 系统输入源菜单共用一份 NSMenu：云联想 / 模糊音勾选、偏好设置、日志目录、版本）
- [x] 退格撤销学习：上屏后整个退格删掉再重打同一段拼音换选，上一次的学习退回去
- [x] 输入日志（2026-09-05）：`input-log.jsonl` 每次上屏一行（键 / 切分 / 前 5 候选 / 选了第几个 / 来源 / 纠错 / 撤销 / 时间），只写本机，
  `[general] input_log` 缺省开，「高级」页开关 + 清空；Core `InputLogger` trait，学习 crate `InputLog` 落盘；离线回放评测与个人模型的数据来源
- [x] 回放评测（2026-09-05）：CLI `--replay input-log.jsonl`，按来源算首选 / 前五命中率、平均名次，打印没命中的例子；内存学习不写文件
- [x] 输入日志 v1（2026-09-12，`docs/plan/model-eval.md`）：日志当评测基础设施，每个模型假设先有尺子再立项。新增事件 `session` / `retype`（组句内退格重打、
  跨上屏相近重打）/ `passthrough` / `prediction` / `break`，上屏行加 `rescored` / `app` / `pages` / `ms`、候选记一页 9 条；空的原样上屏不再记
  （旧日志一半是它）；回放报告加重打、联想接受率、隐式信号（命中 / 未命中两组的翻页与耗时）、重排 / 未重排两组命中
- [x] 整句评测集（2026-09-08）：CLI `--eval-text <中文文本>...`，用户自己的文本切句、按词库读音转全拼、冷启动还原，`--eval-save` 冻结句子集；
  基线（本仓库 docs + 个人技术笔记 8322 句）：首选 37.6%、字准确率 76.9%，技术文本里的行话（候选 / 词库 / 释义表）是主要丢分处
- [x] 删除候选（2026-09-05）：⇧ + 数字（`[shortcut] delete_candidate`），用户词整个删、词库词清全部学习（`Learner::forget`，个人 n-gram `forget_word`），拼音行右侧显示结果
- [x] 拼写纠错：一处编辑（换位 / 换字母 / 多一个 / 少一个）凑出完整音节，拼音行画删除线，回车原样上屏，接受与拒绝都进学习；
  2026-09-05 挑选改成噪声信道（纠正后整句得分扣编辑代价仍高于原样才纠，末尾单字母只试换位），修掉 你后妈 / 我峡谷区 这类选错
- [x] 词图内敲错边 + 个人敲错表（2026-09-06）：整句词图里每个完整音节按一处敲错变体（相邻键换位 / 相邻键 / 多键 / 少键）查词并扣代价，
  `meiganxi` → 没关系；接受的 (敲的, 要的) 音节对记 `user-typos.tsv`，代价按次数打折。回放：带数据 词 91.2% → 91.5%、整句持平，
  冷引擎整句 89.3% → 87.5%（两条歧义句改读：fayibanben 法语版本、shishao 是啥），每键最慢 3.5 ms → 4.7 ms
- [x] README 安装 / 隐私 / 数据许可（2026-09-06）：pkg 安装与「仍要打开」、卸载；不上传数据、云联想发什么发到哪怎么关、
  输入日志缺省开只写本机怎么关怎么清；测试版许可与随包数据署名表（与「关于」页一致）
- [x] 领域词拆成随包的 `.qj`（2026-09-06）：`dict-convert lexicon` 把 THUOCL 领域词按来源文件拆成 11 本（语料 ≥ 50 次的留在基础词库），
  基础词库 22 万 → 8.7 万条、`dict.qj` 10 MB → 3 MB、加载 79 → 25 ms；领域词库合计 13 万条 7 MB 放 `Resources/dicts/`，
  `[dictionaries] domains` 缺省只开成语，偏好设置「词库」页可勾选。回放（614 条冷引擎）：词 前五 96.5% → 96.1%（少一个词），整句持平 87.5%
- [x] pkg 安装器（2026-09-06）：`bundle.sh --pkg` 出 `Qingjian-<版本>.pkg`（装 `/Library/Input Methods/`，关重定位，postinstall 以登录用户身份
  `qingjian-macos --register` 注册并启用输入源，欢迎页 / 许可证 / 结束页），版本号与构建号打包时注入，`uninstall.sh` 随包；
  签名 / 公证做成环境变量开关，等 Developer ID 证书到位就能签。未签名的 pkg 测试者要在「隐私与安全性」里点「仍要打开」
- [x] 崩溃不丢（2026-09-06）：学习数据六张表、`config.toml`、`.env` 原子写（临时文件 + fsync + 改名）；学习数据坏行跳过、
  编码坏了按替换字符读、真读不了退回内存学习不覆盖文件，输入法照常启动；IMK 回调与定时器边界 `catch_unwind`，panic 后缓冲区字母原样上屏；
  激活期间每 60 秒落一次盘。设计见 `architecture.md`「崩溃不丢」
- [x] 候选窗口收尾：竖排 / 横排（`[general] layout`）、拼音行分段样式（Core `MarkedSegment`，纠错删除线的位置已留好）、
  拼音显示位置 行内 / 窗口 / 两处（`[general] preedit`）、按光标所在屏幕定位并处理屏幕边缘
- [x] 日志按天分文件、只留 7 天；文件被删后下一条日志重建（`app/logging/`，自己的 `LogFile` 写入端按 nlink 判断）
- [x] 偏好设置窗口（原生 AppKit，改完即写回 `config.toml`；密钥写 `.env`）
- [x] 偏好设置重做（2026-09-05）：六页标签视图（通用 / 候选窗口 / 快捷键 / 模糊音 / 词库 / 云服务 / 高级），用户视角措辞，
  快捷键录制按钮 + 「恢复默认快捷键」，「词库」页导入 / 开关 / 移除，编辑菜单让 ⌘V 能粘贴
- [x] 产品词库（2026-09-05）：`assets/lexicon/` 自建源（规范字 8105 + 常用词 5.6 万 + THUOCL 领域词 15.7 万），`dict-convert lexicon` 建 `dict.tsv`，
  读音 Unihan + LLM 多音字标注（`gloss-gen pinyin`，6.8 万词，含常用词表自带拼音的 288 处纠错），词频用自己的语料统计；成品 20.5 万条，
  `dict.qj` 10 MB、`lm.qj` 27 MB，启动 70 ms；英文词表换成 ESDB / CSpell 9.5 万词。雾凇拼音与其英文词表已全部移除
- [x] 短语层（2026-09-12）：常用词表是词典词头，不收 我的 / 好的 / 不知道 / 有没有 这类人整块打的组合；`dict-convert phrases` 从语料相邻两三词里挖
  （对话语料 ≥ 2000 次 + 边界规则，读音由成分词拼出）5400 条进基础词库，品牌词 青简 也进（`brand.tsv`）；起因与验收见 `docs/notes/phrase-layer.md`
- [x] 领域词（2026-09-12）：输入日志里选过、公开语料里出现过、词库与短语层都没有的词人工挑 48 条进基础词库（`assets/lexicon/domain_words.tsv`：对齐 / 后端 / 词库 / 候选框 / 看看 / 再也），
  语言模型里与短语一样走合成计数（低频词当 token 统计会吸走成分词的二元证据）；回放词级 +34、整句 +6、整句评测集持平，见 `docs/notes/domain-words.md`

2026-09-06 状态：自用日常在用；`bundle.sh` 从 `assets/`（词库源、释义表）与 `data/generated/`（生成物、`.qj`、领域词库）打产品数据，没有生成物时打样例；
`--pkg` 出分发包。给测试者发第一版前只剩特殊应用验证；签名等 Developer ID 证书。

## Phase 3 — Translation

- [x] Candidate Translation API（`Translator` trait，`Engine::annotate` 候选生成后异步补译文；`Sense` 词性 + 译文 + 读音）
- [x] 中文 → English、中文 → 日本語：`tools/gloss-gen` LLM 批量生成 6.4 万常用词（词性、译词、日文假名），随包表；日文按汉字段注平假名
- [x] 本地词典：`Glossary` 内存查表，格式 `词\t词性. 译词[|假名]`；CC-CEDICT 表保留为备用来源
- [x] 翻译语言设置：`[general] learning_language`，菜单与偏好设置只列打进包里有释义表的语言
- [x] Option+数字上屏候选的译文
- [x] 释义表随仓库发布（`assets/glossary/`，2026-09-05）：中→英 / 日覆盖整个自建词库 24.9 万词，多字词 91.6% 有英文释义、98% 日文；
  英→中 4.5 万词（wordfreq ≥ 2500 + 技术词），英文候选右侧显示中文（Engine 按候选种类查表）；三张表都进 `.qj`
- [x] 上屏译词：⌥ + 数字第一条、⇧⌥ + 数字第二条，修饰键可录制；⌃⌥T 翻译应用里选中的文字（云服务）
- [x] 释义兜底（2026-09-06）：Core `GlossFiller` trait（与 `Predictor` 分开：独立线程、攒批、不丢请求），随包表查不到的词库词 / 云端词**上屏后**入队，
  `qingjian-predict::CloudGlossFiller` 攒 1.5 秒或 8 个词发一次（提示词与 gloss-gen 同源，只要当前学习语言），结果经 `Translator::learn` 进
  `qingjian-translate::PersonalGlossary`（`user-glossary-<语言>.tsv`，格式同随包表，可手改，`LayeredTranslator` 个人表优先），随 flush 落盘；
  壳每秒 `Engine::poll_glosses`；随云联想开关一起开，发出去的只有那个词
- [ ] JMdict 校对假名与日文词性（等级标签已做，见 Phase 4）

## Phase 4 — Learning

- [x] 用户词频（`user.tsv`）、用户词（`user-words.tsv`）
- [x] 词级排序接语言模型：上一个上屏的词做上下文，同输入串下选过的词优先（`user-choices.tsv`），非末尾简拼少的切分优先
- [x] 输入习惯学习：个人 n-gram + 自动造词；用户点选的转移记双份、整句路径顺带的记一份，选择次数加分取对数并封顶（否则 的 的 173 次会把 号的 抬过 好的）
- [x] 个人敲错表（2026-09-06，`user-typos.tsv`）：接受过的 (敲的, 要的) 音节对，词图敲错边与整段纠错按次数打折；退格撤销一并撤销
- [x] 学习数据落盘可靠（2026-09-06）：六张表原子写、坏行跳过、读不了退回内存学习、激活期间每 60 秒落一次盘（Phase 2「崩溃不丢」）
- [x] 输入统计（2026-09-06）：Core `UsageMeter` trait，每次上屏折成汉字 / 中文词（整句按切词数）/ 英文词 / 次数；
  `qingjian-learning::UsageStats` 按天记 `usage.tsv`，与输入日志无关（日志关掉或清空统计还在），随学习数据每 60 秒落盘；
  偏好设置「统计」页：今天 / 最近 7 天 / 累计 三行四列 + 「累计 xx 字，约等于 n 本《某书》」（`book_scale`，书目从《道德经》到《平凡的世界》）
- [x] 生词识别（2026-09-06）：Core `VocabularyTracker` trait + `Sense::fresh`；一条译词在用户上屏时出现在候选窗口里的轮次不到 3（`FRESH_UNTIL`）算生词，
  候选里橙色画译词，看熟了变回灰。「看到」按上屏那一刻屏幕上那一页算（壳 `Engine::note_displayed`），上屏带译词的候选记「上屏过」，⌥+数字 打出译词记「用过」；
  `qingjian-learning::VocabularyBook` 一个译词一行 `user-vocab.tsv`（语言 / 译词 / 看到轮次 / 上屏 / 用过 / 首见 / 末见），只记译词与次数
- [x] 学习词汇统计（2026-09-06）：偏好设置「统计」页词汇块：见过 / 看熟 / 上屏过 / 打出过 / 本周新见的译词数（`VocabularySummary`），
  再按等级分行（词表总数 / 见过 / 看熟 / 上屏过）：英文 CEFR A1–C2（CEFR-J 1.5 + Octanove，8,845 词）、日文 JLPT N5–N1（Tanos 经 elzup，7,757 词），
  `qingjian-translate::LevelTable` 读 `assets/levels/levels-{en,ja}.tsv`（`tools/corpus/levels.py` 生成），`VocabularyBook::with_levels`；等级不进候选窗口
- [ ] 可选复习功能

## Phase 5 — Cross-platform

- [~] Windows TSF（`apps/windows/{server,tsf}`，真机自用中；细节见 `docs/design/architecture.md`「Windows：TSF」）
  - [x] Server 进程 + TSF DLL 骨架、命名管道 IPC、多会话分派、端到端上屏
  - [x] preedit 内联下划线、候选窗（词性 + 译文 + 分页 + 阴影）、云联想、失焦上屏、中英切换、设置界面、Inno 安装器
  - [x] 候选窗渲染搬进 Server 进程 + `uiAccess` + 自签，覆盖微软商店 / 任务栏搜索等高 z-band 宿主
  - [ ] 发版：Certum 开源代码签名证书、`windows-v<版本>` 标签与 CI
- [ ] Linux IBus / Fcitx
- [ ] 配置同步
- [ ] 跨平台词库

## Phase 6 — 云联想

Core 定义 `Predictor` trait（与 `Translator` / `Learner` 同一模式），网络实现放独立 crate `qingjian-predict`，
Core 永远不联网。第一个实现接 DeepSeek（OpenAI 兼容接口），接口做成 provider 无关。

- [x] 本地输入历史：`InputHistory`（内存环形 4096 字符，可清除；落盘与查看界面待做）
- [x] `Predictor` trait：`submit` / `poll` 非阻塞，请求带序号，Engine 只认最新序号；`NoPredictor` 缺省
- [x] `qingjian-predict`：`CloudPredictor`，async-openai 走 OpenAI 兼容接口，后台线程防抖 300 ms、缓存 64 条、超时 5 s
- [x] 云端词：模型给「拼音对应的词 + 全拼」，Core 逐音节校验；词库没有的选中后记成用户词（`user-words.tsv`）
- [x] 云端词位置（2026-09-05）：到了只补进第一页末尾 `[predict] slots` 格（缺省 2），前面的本地候选一格不挪，不预留不画占位；
  Core `CandidateLayout` 出排布，壳 `Session` 只管高亮与页码；偏好设置「云服务」页可选 0–4 格
- [x] 触发：只在组句中，停键后同一次请求要云端词和整句补全（preedit 右侧，Tab 接受）；上屏后不联想
- [x] 整句补全进学习（2026-09-05）：Tab 接受的句子按语言模型切词（`sentence::segment_text`）记进个人 n-gram，可随退格退回
- [x] 纠错：云端词按容错编辑距离校验（简拼不算错），模型可纠正错字 / 漏字 / 多字；本地历史不进请求
- [x] 提示策略（2026-09-05）：本地候选与本地整句在提示词里标成「可能全错的本地猜测」，模型独立判断、只给本地给不出的词，没有就空；不再照抄本地候选
- [x] 上下文来源：IMK `attributedSubstringFromRange:` 读 marked text 前后，应用不支持时退回本地输入历史
- [x] 展示：云朵图标（SF Symbols `cloud`）区分，**不重排已有候选、不阻塞输入**
- [x] 隐私：默认关闭；开启时菜单栏「中 / 英」带 ☁︎；Secure Input 绝不发送；观察长度在 Core 裁剪
- [x] 配置：`~/Library/Application Support/Qingjian/config.toml` 的 `[predict]`，首次运行写模板；密钥可填配置或环境变量
- [x] 问字模式：`?` + 问题拼音，`PredictionKind::Question` 走同一条 Predictor 通道、单独的提示词，答案带声调读音、不校验拼音（见 candidate-ui.md）
- [x] 问字只答字不复述（2026-09-05）：请求带本地整句转换出的问题汉字（`guess`），提示词只答被问的字，`restates_question` 剔掉把问题写回来的「答案」
- [x] 菜单开关、偏好设置窗口里的开关 / 接口 / 模型 / 密钥
- [x] 释义表进 `.qj`（2026-09-05）：`Glossary` 双存法（TSV 哈希表 / 映射的 arena + 词条表 + 释义表 + 哈希索引），`pack glossary --language`，启动回到 50 ms
- [x] 翻译选中文字读不到选区时候选窗口提示 2.5 秒（`host/notice.rs`）
- [x] 语料挖新词（`dict-convert mine`）：分词落成连续单字的段按子串计数，虚词规则 + 相邻字对 PMI≥3 过滤（2026-09-07 从会话脚本进工具，`oov_filter.rs`），`lexicon --extra-words` 并入词库
- [x] 多词库与词库管理（2026-09-05）：Engine 附加词库列表；用户目录 `dicts/` + `[dictionaries] disabled`；偏好设置「词库」页导入（TSV / Rime yaml / .qj → .qj）、开关、移除
- [x] 翻译选中的文字（2026-09-05）：⌃⌥T 把应用里的选区交给云端翻译，译文在候选窗口里回车替换、Esc 保留；快捷键可改；
  双向：中文选区译成学习语言，外文（拉丁字母 / 假名）选区译回中文，方向按字符统计定（`translation_target`）
- [x] 输入历史落盘与清除（2026-09-05）：`input-log.jsonl` + 偏好设置「高级」页开关 / 清空（查看界面没做，文件是 jsonl 直接看）
- [ ] 密钥进钥匙串（等签名定了再做）；云联想按应用禁用
- [ ] 请求量控制：按分钟限流、统计本月请求数并显示

## Phase 7 — 个人模型

目标是「输入法越用越像自己」。分两步，第二步有评测门槛。

- [x] 个人 n-gram：bigram 计数 + 与静态模型按证据量插值，随上屏在线更新，直接进整句转换的 Viterbi 打分（见 Phase 2）。
  个人数据量级（一年几百 KB 到几 MB）上，这一步拿到绝大部分个性化收益，成本接近零
- [x] 个人 trigram（2026-09-06）：与二元一起在线计数，绝对折扣（D = 0.75）回退到二元，不训练；`--replay` 在 423 条日志上与二元持平
  （冷 词 77.7% / 整句 87.0%，带数据 90.2% / 97.1%），三元要数据攒起来才见效，`--typing` 每键最慢 2.7 ms 不变
- [x] 小 Transformer 实验分支（2026-09-08）：`qingjian-neural`（candle，Metal / Accelerate / CPU），用途「重排整句转换的前 6 条路径」；
  CLI `--neural` 接，壳未接。回放上词 +0.5、整句 −1.1（静态替换 λ 0.25、前文 64），没过门槛，且尺子有偏，见 `docs/notes/neural-rescoring.md`
- [x] 整句评测集上过门槛（2026-09-08 晚）：冻结集 8322 句，基线整句首选 37.6%，small λ 0.75 前文 64 到 41.9%（+4.3），
  不给前文也有 +2.2；翻好 382 句、翻坏 54 句，见 `docs/notes/neural-rescoring.md`
- [x] 进壳（2026-09-08 晚）：前文 KV 缓存（64 字前文 × 8 条 133 → 28 ms）、Core `engine/rescoring` 异步重排（后台线程 + 文本分数缓存，壳停键 80 ms 请求）、
  应用光标前文当前文、λ 缺省 0.5、`[model] enabled` 开关与「云服务」页勾选、模型随包放 `Resources/model/`；TextEdit 端到端 候选声称 → 候选生成
- [x] 模型单文件 `.qjm`（2026-09-12）：`.qj` 容器新种类 `Kind::Model`，三节原样装三件套；`find_model` 先 `.qjm` 再目录；`pack model` / `tools/release/pack-model.sh`；
  data Release 上传 `model.qjm`，mac `Resources/model/` 与 Windows `data\model` 只带这一个文件
- [ ] 本地模型后续：模型加载 12 秒（后台，首次 Metal 编译）看能不能缩；权重许可已定与代码一致 GPL-3.0-or-later（2026-09-12，写在 `pack-model.sh`）；个人微调（闲时训练）
- [ ] 闲时训练：门禁包括接电源、温度、空闲时长；训练数据来自本地输入历史；模型与数据都可一键清除
- [ ] 评测门槛：留出用户文本上比 n-gram 的困惑度与 top-1 命中率，赢了才默认启用；每次按键推理延迟有上限
- [ ] 模型文件的版本与迁移

## 规模参考

| 项目 | 量级 |
|---|---|
| librime（C++） | 约 5 万行 |
| Phase 1 + 2 到 macOS 自用级别 | 全职约 2 到 3 个月 |
