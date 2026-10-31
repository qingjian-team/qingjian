# 各 crate 的实现要点

`qingjian-offline`、独立选区翻译提供方、Windows 接线及模型下载见 [原生离线选区翻译](native-offline-translation.md)。

CLAUDE.md 只保留目录地图与规则，每个 crate / app / tool 的实现细节收在这里：入口类型、数据文件、常数、生成命令。
改了实现要同步改这里；与代码冲突时以代码为准。

## crates/qingjian-dictionary

词库（TSV 解析或 `.qj` mmap），键按字节序排好，查询逐音节位置二分收窄（简拼位置按音节块跳扫），
`lookup_pattern`（≥ 模式长度）与 `lookup_exact`（正好等长）同一套实现。词库键以 `v` 表示 ü，
TSV 解析、查询与生成工具把 `lue` / `nue` 统一成 `lve` / `nve`。
英文词表 `WordList`（`english.tsv`，9.5 万词）编码与原样词拼进一块缓冲、条目只记位置，按编码二分：每词三个 `String` 加 `HashMap` 时常驻 18 MB，现在约 4 MB（2026-10-28 探针）。
旧 `.qj` 含这些键时，加载器建立规范化的内存词库。新 `.qj` 继续使用 mmap。
TSV 的音节可带数字声调（`ni3`，轻声 5 或 0），键一律去调，声调只给显示（`Match::tones`，`tones.rs`）：五个音节以内压进 `Slot` 原来的 16 位填充（每音节 3 位，0 是不知道），
多音字的读音占比：加载时走一遍单音节的键，记下有不止一个读音的字全部读音的词频之和（`dictionary/readings.rs`，不落盘），命中带上 `Match::reading_total`，`Match::reading_share` 是这个读音的词频占比（`区 ou` 约 0.05）；多音节的词不查表。
更长的在那 16 位放溢出标记、声调进 `TSPL`（按词目下标升序，二分找）+ `TONE`（每音节一个 `0`–`5`）两节；没有长词带声调的词库与之前的旧文件没有这两节，旧文件填充是 0，读出来全是不知道。
显示：`[general] show_tones` 开着时候选窗口顶部的拼音行按高亮候选标声调（`Engine::toned_syllables` 出逐音节带调拼音，`engine/reading.rs`；`Query::window_segments` 拼行，读音对不上敲的不换），应用里的 marked text 不变，帧里另给 `Frame::window_preedit`（DLL 不读）。整句从左往右取词库里最长的词，词库没标的字按单字声调表 `zhuyin::tone::usual` 补（首读音，或这个音只有一种声调），拿不准的不标；注音、九宫格不标。Linux 的 Fcitx 面板与行内拼音共用一串，还没接。
`Dictionary::set_secondary` 标补充词库（不落盘，装配方标；随包的通用大词库 `dicts/general.qj`，文件名主干是 `GENERAL_DICTIONARY`），命中带 `Match::secondary`。
辅码码表（`aux_code_table/`，`AuxCodeTable` = `Kind::AuxCodeTable` 的 `.qj`）是另一套存储：`TEXT` 词 arena + `CODE` 码 arena +
`ENTR` 条目（12 字节，词字节序升序、同词相邻）+ `HASH` 词 → 条目区间起点（`qingjian_format::hash`）。查询只有
`AuxCodeLookup::code_with_prefix` 一个方法：候选词逐个问「有没有以当前码段开头的码」。导入 `import_aux_code_table`
解析 Rime `.dict.yaml` 的 `columns`（缺省 text / code / weight）与 `import_tables`（相对主文件目录合表），
「有词无码」与非法码进统计返回、不静默跳过；一个可用条目都没有时报 `NoCodeEntries`。
`WordRules`（导出为 `AuxWordRules`）是辅码方案的词码规则，`AuxCodeTable::with_word_rules` 挂上、不落盘：表里没有的多字词按词长挑公式（Rime `encoder` 写法，`A`–`T` / `a`–`t` 正数、`U`–`Z` / `u`–`z` 倒数，最长那条也管更长的词）从单字码拼，
字有多码时按组合出多条（上限 16）；所以 `AuxCodeLookup` 返回 `Cow`，推出来的码是新串。

`CodeTable` 是形码码表（五笔），与词库并列的另一类查表：键是编码本身（`ggll`），不是拼音音节序列，
格式 `词\t编码\t词频`（随包五笔 `assets/schemes/wubi86/table.tsv`，8.9 万条），按编码排序、`lookup` 二分定位前缀区间。
编码打全的词（`exact`）排在同前缀的更长编码词前面；单字母精确码按码表行序排列（五笔生成器保留上游行序），其他命中仍按词频排序，
引擎在上下文排序后恢复这组单字候选的码表顺序。二级 / 三级简码仍是编码前缀查询的自然结果。
与词库的一处关键差别：码表没有 `.qj` 容器，只读 TSV。

## crates/qingjian-core

`Engine::set_emoji_limit` / `set_kaomoji_limit` / `set_symbol_limit` 控制单次查询插入的 emoji、颜文字、符号数，各自 0–3（缺省 3 / 1 / 1）、0 关闭、越界值按 3 处理；每词 emoji 与符号最多两个、颜文字一个，普通候选相对顺序不变。三端读 `[general] emoji` / `kaomoji` / `symbols`（`qingjian_platform::EmojiCount`，旧配置 emoji 的 `true` / `false` 读成 3 / 0），CLI 用 `--emoji-limit <0..3>`。
`with_emoji` 带上 CLDR 表时用 `emoji::is_emoji`（Unicode `Emoji` 属性，BMP 码位表取自 emoji-data.txt 15.0）筛掉 `≈` `¥` `«` 这类普通符号，并上 `crates/qingjian-core/data/emoji.tsv` 的口语叫法，同时启用同目录编进程序的 `kaomoji.tsv`、`symbols.tsv`（一行一条 `词\t条目`），以及 `words.tsv`（这些词里词库没有的，带读音，排在附加词库之后、用户词之前一起查）。符号表的中文词只收两个字以上的（有测试守着）。
`EmojiTable` 的条目拼进一块缓冲、词只记序号区间，`lookup` 返回 `&str` 迭代器：每个 emoji 一个 `String` 时两张随包表常驻 3.5 MB，现在约 1.6 MB（2026-10-28 探针）。
`emoji::skin_tone_variants` 给面板里能接肤色的 emoji 列五档（修饰符接在第一个字符后、去掉紧跟的 FE0F；序列里有两个以上能接肤色的字符不给），`Emoji_Modifier_Base` 码位表取自 emoji-data.txt 15.1。面板目录本身仍只收不带肤色的那一条。
`email::email_completions` 给手机端「用户名@」之后列常见邮箱域名（`email/mod.rs` 的 `DOMAINS`，12 个），按 `@` 后已敲的前缀过滤、不分大小写，`insert` 是去掉已敲部分的余下那截；用户名只认 ASCII 字母数字与 `._-+%`，前面可以紧挨汉字。桌面端由 `query_raw` 接：英文直输段是「用户名@」时原文后面跟整个地址（`Shortcut` 候选，不记学习），壳在直输段里按空格选中地址时不再把空格交给应用。
`calculation::calculation_completions` 给手机端「刚上屏了一道算式」时的结果候选：光标前末尾一段由数字、`+ - * / × ÷ ^ ( ) .` 组成、前面不紧挨 ASCII 字母，至少一个运算符夹在两个数之间就交 `shortcut::evaluate` 算；末尾带 `=` 只给结果，否则给「换成结果」（`replace` 是要先删掉的字符数）与「补上 =结果」两条。只用 `-` 或只用 `/` 连两次以上的（日期、电话）和带前导零的数（区号）不算。全拼出了「日期 / 时间 / 星期」（前 `KEYWORD_SCAN` 个词，emoji 不算）时 `insert_keyword_shortcuts` 把 `rq` / `sj` / `xq` 那组写法插在这个词后面、它的 emoji 之前；九宫格的数字串不认 `rq`，拼音列把字母都定下来（`r'q`）才走 `insert_shortcuts`。
面板「常用」按用户自己用的排：`panel::common_items(group, uses)` 把用过的（次数多的在前，同次数近的在前）提到内置常用表前面，只提一格放得下的（`NARROW_EMS`，`.com`、颜文字不提），带肤色的 emoji 不在目录里也算；次数由 `qingjian_learning::PanelUsage` 记在 `panel-usage.tsv`（`条目\t次数\t最后 Unix 秒`，最多 300 条，每用一次整个重写）。Android 壳直接读写 `PanelUsage`；桌面三端由 Engine 经 `PanelHistory` trait 注入（`with_panel_history`，`PanelUsage` 实现，缺省 `NoPanelHistory`）：`SymbolPanel` 打开时用它重排符号组与表情组的「常用」，面板开着时 `commit` 的条目记一次（私密输入不记）。
`sentence::text_pieces` 把任意文本切成可点选的小块（剪贴板「拆词」），**全部拼起来正好是原文**：汉字段按 `segment_text` 切词（模型一个词都不认识就一字一块）；空白也是块（一串空格制表符一块、换行单独一块）；不带空白的 ASCII 串里有 `/ . @ :` 且有字母的（链接、邮箱）去掉首尾括号后整块，数字段之间夹 `- . , : / +` 的算一个数（电话、日期、时间、千分位，可带开头的 `+`），其余字母数字一块、标点一个一块，连着的同一运算符（`//` `==`）合一块。
`code::looks_like_code` 认出像代码的文字（至少两行，有缩进或括号分号成堆，剪贴板等宽显示用），`code::highlight` 粗切高亮片段（关键字 / 字符串 / 注释 / 数字，字节范围）：不认语言，`//` 与行首 `#` 与 `-- ` 行注释、`/* */` 块注释，三种引号的字符串（单双引号不跨行、找不到收尾就不算，挡住 Rust 生命周期与英文缩写），数字不能紧跟标识符，关键字取常见语言的并集。
日语罗马音（`japanese/`，目前只给 Android 测试用）：`to_hiragana` 最长匹配转平假名（同辅音连打 → っ、`n` 后接辅音 → ん、`-` → ー，没拼完的罗马音留在末尾），`RomajiComposer` 出候选（汉字词、平假名、片假名、边敲边出的汉字词、原样罗马音）。汉字只查 `words.rs` 里约 130 条手写常用词，正式的词典与转换以后再做，那时删掉这张表。
符号面板在 `panel/`：`SymbolPanel` 管分组与分类切换，`Engine::open_symbol_panel` 拿当前前 12 个候选词查 `keywords.tsv`、emoji 表与内置符号 / 颜文字表当搜索结果；
条目是 `Emoji` / `Kaomoji` / `Symbol` 种类、`reading` 为中文名的候选，上屏走 `commit`（没有音节，吃掉整段）并关掉面板。格子移动复用 `Grid`。
macOS 面板一律系统绘制（青简渲染器还没有面板），`candidates/view/panel.rs`；开着时云端词与模型重排不往格子里写。
一行几格由 `PanelSection::columns` 按最宽条目定（`PANEL_COLUMNS` = 10，颜文字 6 或 4）。Windows 的面板状态在 Server 的 `dispatch/panel/`（格子 `PanelGrid`，按键 `dispatch/key/panel.rs`），
窗口在 `ui/panel/`，用 Direct2D DC 渲染目标画在分层窗口合成器的 DIB 上（GDI 画不出彩色 emoji）；渲染目标是软件的，硬件类型要起 D3D 设备，显卡驱动初始化就常驻二三十 MB。没在组句时从菜单打开：协议 `Frame::symbol_panel` 让空缓冲也不算空帧，
`IndicatorState::symbol_panel` 随 `ModeSync` 让 DLL 进组句态吃键；老 DLL（协议 < 9）收到的颜文字 / 符号候选降级成 `Emoji`。
双拼解码由 `Engine::decode` 传入当前模糊音规则：先取普通合法音节，失败时才保留能由模糊音补成合法音节的声韵组合（如 `suang` → `shuang`）。查询沿用带代价的模糊音展开，上屏沿用原读音与按键的消耗换算；不修改无模糊音时的 `Scheme::decode` 行为。
`Engine::decode_readings` 在开了 `[general] shuangpin_full_pinyin` 时给出一段键的多条读法（`shuangpin::Layout::decode_mixed`）：纯双拼、整段全拼、以及按音节边界交替的读法，
每个位置最多留 `MAX_PATHS`（8）条部分读法；主读法管 preedit / 尾巴，候选覆盖长度换算成敲的键数再排序，整句候选取各读法路径分最高的一条。上屏与整句回灌按候选音节挑对得上的读法换算键数与学习键。
`engine/commit/mod.rs` 只管上屏主流程；键串与候选音节的对齐（`align`、回溯与敲错变体）拆在 `engine/commit/align.rs`。

CandidateLayout 保持 `set_cloud` 的首页末尾布局与本地首项；`set_cloud_first` 为显式请求提供模型顺序置顶并合并同文普通候选。自定义短语按名次固定的格子保留；`cloud_first()` 供异步本地重排重建布局时沿用当前模式。自动入口及默认配置的 slots 仍指首页末尾。

候选右键菜单可修正学习语言译文：手工修改由 macOS / Windows 原生输入框收文字，`Engine::set_candidate_translation` 写入个人释义表并立即 flush；模型重生成使用 `PredictionKind::CandidateTranslation` 和 `Engine::request_candidate_translation`，目标语言取当前 `Translator::language`，专用词典提示请模型同时返回词性；解析层有可识别词性时规范成 `词性\t译词`，漏了或认不出词性时保留纯译词。`Engine::set_generated_candidate_translation` 只替换第一条译词：模型给了合法词性才更新原词性，原有读音和后续释义都保留；原来没有释义时才新建一条。个人释义优先于随包释义；私密输入、未选学习语言或 Predictor 未接入时不发。

模块：`composition`（缓冲区与光标；中文模式下 Shift+字母按小写进 `buffer` 参与匹配、大写记在 `shifted`，`typed_text` 还原后用于原样上屏）/ `parser` / `correction`（拼写纠错：整段一处编辑的候选纠正 + `typo` 音节级敲错变体表，后者进整句词图当带代价的边）/
`candidate` / `ranking` / `shortcut` / `sentence` / `fuzzy` / `shuangpin`（双拼：`Layout` 是键位表 + 键 → 全拼解码与消耗换算，九套内置方案的常量表在 `table.rs`、`Scheme::layout` 取出，用户方案由平台层读 `scheme.toml` 拼成 `Layout`；整串一个音节都配不出时 `decode_fuzzy` 按每键一个声母再解一次（`im` → sh'm → 什么，`decode_initials`），能正常解出的输入不走这条；`Layout::check` 报写错的键与打不出来的音节——按全部两键组合实际解出哪些音节算，不是按能不能编码；`key_hint::KeyHint` 按 `Layout` 画键位提示，用户方案也画自己的键）/ `root_chart`（86 五笔字根图，字形表 `assets/schemes/wubi86/roots86-glyphs.tsv` 编进 Core，`Engine::root_chart` 只在挂着 86 五笔时给，别的形码方案不画）/ `zhuyin`（大千注音：键 → 注音符号 → 拼音，`[general] zhuyin` 开关；声调判音节完整，不进查询、只进排序：`zhuyin::tone` 按单字声调表 `data/tones.tsv` 给每个候选算声调代价——字的首读音 0、次要读音或表里没有 1、对不上 16，多字词非首字的轻声不计——排序键里排在「末音节完整」之后、「同输入串选择次数」之前，全拼 / 双拼恒为 0；整句词图同样用它：格子选词按 `sentence::TONE_PENALTY`（每点 ln 2）打折、路径得分扣同样的分，格子缓存键带上声调（`SpanCache::toned_key`），只在要转的音节正是当前注音输入解出的那串时才给声调，纠错与英文尾段换过切分的不给）/ `emoji` /
`english`（英文模式候选）/ `traditional`（繁体输出：`[general] traditional_variant` 四选一，`off` / `taiwan` / `hongkong` / `standard` 对应 OpenCC 的 s2tw / s2hk / s2t，都只换字形；旧键 `traditional = true` 读成 taiwan，开发版在旧键里写过的字符串（`traditional = "off"`）按同一套写法读。新写法换键名是为了退回旧版本时旧版只认布尔、不会整个配置解析失败，字典由 `ferrous-opencc` 在编译期编进产物、运行时不读外部文件）/ `engine`（`query::EnglishTail`：句末英文词并入整句，`woxiangxuehaorust` → 我想学好rust，尾段也像拼音时按分数与拼音读法比；比分与第二条整句里的拼音读法都按整句的切分规则挑——`sentence_segmentation`，不是切分器打平时贪心的那种，否则 `zhengcenanti` 拿 zheng cen an ti 的 政策安提 去比，政策anti 赢、政策难题 连候选都没有）。
辅码（`engine/aux_code.rs`）：`Engine.aux_code: Option<String>` 是码段（`None` 拼音态，`Some("")` 刚触发或删空停在辅码态——`Engine.aux_keep_empty`，配置 `[general] aux_code_keep_empty` 缺省开），
不进 `Composition`；`aux_trigger`（配的触发键 + 光标在段尾 + 作用域能完整切分 + 双拼韵母键优先）、`enter_aux`、
`push_aux_code`（只收 a-z）、`clear_aux`；退格在辅码态内部分派（删码；删空按 `aux_keep_empty` 停在辅码态或回拼音态，空码段再退格退出），`commit_with` / `take_raw` /
`punctuate` / `clear` 都收尾清码。查询在 `rank` 之后**反向过滤**：候选词逐个 `AuxCodeLookup::code_with_prefix`
（码表在 `Engine.aux_codes`，`set_aux_codes` 注入），不命中的隐藏，命中的按「完全匹配码 > 码长降序 > 原词频序」
重排（stable sort 保住原序），命中码（没在筛码时是词的首条码）写进 `Candidate.aux_code`；码段非空时跳过整句 / 英文 / 快捷 / emoji / 自定义短语
与云联想。preedit 分段多出 [触发键 `Typed`][码段 `MarkedKind::AuxCode`]，见 `Query::marked_segments`。
拼音行按哪种切分画是 `Query::spelled`：缺省对着首选候选挑（与候选音节逐个相同的最多，其次是候选音节以敲的开头），壳换了高亮调 `Query::follow`（`ganga` 高亮 尴尬 是 `gan'ga`、刚啊 是 `gang'a`）；
mac 存在 `Session::spelling`（`Host::follow_highlight`）、Windows / Linux 存在 `Composed::Candidates.spelling`，出帧时比一下高亮候选的切法，不同才重拼。Android 没有高亮移动，只跟首选。

双拼 `Scheme::syllable` 优先解出键位表里的合法音节；解不出且该韵母键含 `v` 时，j / q / x / y + ü 规范化为 ju / qu / xu / yu。只补此前无效的组合，不覆盖微软 `jv` → jue 等既有解码。

形码在 `engine::query::code`。`Engine` 上有两个开关：`set_code_scheme`（`CodeScheme` = 码表 + 按键规则 + 日志键）与 `set_phonetic`（拼音侧参不参与），
在 `query_inner` 进切分之前按这两个分派——只有拼音 / 只有形码（`query_code`）/ **两边都开（`query_mixed`，混输）**。
编码按前缀查表，`CandidateKind::Code` 的候选 `syllables` 为空、上屏吃掉整段作用域（`whole_scope`）。
混输是「拼音那条 Query 前面插上形码候选」：拼音读不出来时（`ggll`）整个按形码走，两边都空才算错；
按文本去重，编码打全的形码词在前、拼音居中、只命中前缀的形码词垫后（一律形码在前的话 `kai` 的首选会变成编码 `kaik` 的词）；混输下模式键同双拼换成大写，满码长度（`CodeScheme::code_length`，缺省取 `CodeTable::full_code_len` 即最常见的编码长度，五笔是 4；极点码表里有条写错的 5 码 `TF卡 ttfhh`，取最长会把满码拖成 5）以内不做拼写纠错，超出码长自然只剩拼音。
只用形码时还有两条按键规则（`engine/code_keys.rs`，混输下都不生效）：`code_tops(c)` 判断满码再敲字母要不要先顶字，由壳上屏**高亮**候选再收这个字母；
`commit_unique_code()` 在壳每收一个字母后调，全码满码且码表里只有一个词时 Core 自己按首选上屏（走完整 `query` 拿出 Core 的那份候选，繁体输出照样对）。
`composing_code()` 告诉壳「单用形码、缓冲区是一段编码」：壳据此让会转全角的标点总是先上屏高亮候选（不看 `punct_commits`），
编码查不到词（空码）时 `commit_highlighted` 清掉编码而不是原样上屏字母（回车仍原样上屏）。
码表里编码不全是字母的行在 `CodeTable::parse` 时跳过（形码查询只收小写字母）。
拼音那套在纯形码下全部不适用，靠 `modes()` 返回 `ModeKeys::LETTERLESS` 与 `active_correction` 直接返回 `None` 关掉；
译词标注、生词记录、输入日志、用户选择学习与个人 n-gram 仍照常工作。
`Engine` 是对外唯一门面，`Translator` / `Learner` trait 在 `engine` 模块；词库是「主词库 + 附加词库（`set_extra_dictionaries`）+ 用户词」的列表。
补充词库（`Match::secondary`，通用大词库一百多万条、不少是语料片段）低主词库一档：词级只查音节数正好对上的（`lookup_all`，不当前缀补全）、精确命中不进排序第一档（`Scored::first_tier`，与主词库的前缀词、敲错纠正一起按打分比）、兜底与词图格子预选再扣 `SECONDARY_PENALTY`、不计入总词频；整句那边「整段就是一个词就不走敲错路径 / 不出整句」只认主词库的词（`in_primary_dictionary`），词图选中的补充词库整词或撞上补充词库词的整句照样插到最前。扣分 1–4 冻结回放只差两三条，取 2。繁体输出是 `Traditional`（`crate::traditional`）：`Engine::query` 出口把候选换成繁体并记下「繁体 → 简体」，云端联想词与整句在 `poll_prediction` 那条路上走 `convert_remember` 单独转，`commit` / `annotate` / `forget` / `accept_prediction` 按这张表还原成简体，表在组句边界 `forget` 清；所以词库、词频、用户词、输入日志一律是简体。`LastCommit::chars` 按上屏的繁体算。
词级排完序后 `ranking::arrange` 按类别重排一次（各类内部不动；这个输入串下选过的不挪），输入不止一个音节时按「句子 > 词 > 单字」分段、不穿插：词至多 `WORDS_BEFORE_SINGLE_CHARS` = 6 个（盖住整段、读音就是敲的音节的词，主词库在前、补充词库的跟着 → 至多 `COMPLETIONS_BEFORE_PREFIXES` = 3 条猜测：多出音节的补全、把打完的末音节读成更长音节的词，只在输入有一种全是完整音节的切法时算猜测 → 只盖住开头一段的主词库前缀词）→ 首音节单字（头 `SINGLE_CHARS_BEFORE_SPLIT` = 9 个之后接拆开完整音节才读得出的 `Scored::split`：`shishan` 按 `shi s h an` 读的 事实；没有词读音正好是敲的音节时它们多半是手误或没打完，`wodei` 的 我的，按词算）→ 其余的词与猜测 → 补充词库的前缀词（你邮、泥涌 这类凑不成话的）。6 个是按冻结输入日志定的：多音节输入选多字词 99.8% 在前 6 位，选单字的却有 23% 在第 9 位以后；搜狗也是几个词之后全是单字（`ceshi` → 测试 侧石 侧视 策士 侧室，接着 测 侧 册）。末尾只敲了个没打完的音节（`woganj` 的 j，且没有全是完整音节的读法）时，盖住前面完整音节的词（我敢）也算整词。2026-10-28 冻结回放（无模型）：词首选 8202 → 8203、整句首选 2769 不变、整句平均名次 1.89 → 1.58；知意：词 8301 → 8300、整句 2662 → 2660；开发集、回归集（#769 的单字上限放宽到第 18 位）都不变。原来按「覆盖字母数」排最前，拆开 `shan` 读出的两百多个词与五十个 时尚X 补全把单字挤出前 500 条（`shishan` 一个单字都没有）。2026-10-26 冻结回放（知意均衡档 + zh.ngram）：词首选 8322 → 8319 条、前五 97.3% → 97.6%、平均名次 1.51 → 1.38，进第一页 51 个、掉出 12 个（多掉到第二页；我 105 → 5、个 85 → 4）；社区开发集不变。用户报过的例子在 eval-dev 的回归集 `sets/regression.tsv`。

模糊音命中：末音节读成它的模糊音写法（`FuzzyRules::readings`，`zi` 开 z-zh 读成 zhi、`fan` 开 an-ang 读成 fang）也算「末音节完整」，扣 `FUZZY_PENALTY` = ln 2（`fan` 按前缀查到的 fang 本来不扣，当模糊音读法时补扣）。单靠扣分时单字间词频差得远，`zi` 首选会是 只，所以输入只有一个音节时 `arrange` 再把模糊音命中的单字排到原样读音的头 `SINGLE_CHARS_BEFORE_FUZZY` = 9 个单字之后（选过的不挪）；多音节输入的模糊音词照得分排（`lihao` 开 n-l 首选仍是 你好）。原来末音节只认敲的原文，模糊音命中的单字全排在原样读音的几百个生僻字之后（`zi` 的 知 第 220 位，`li` 的 你 被预选砍出前 500）；改后在第 10–19 位。模糊音默认全关，冻结回放、开发集与回归集都不开，2026-10-29 三组（无模型 / 知意 / 通变）前后一致。
读音占比进打分：语言模型按字形计数，`区` 的 qu 撑起来的分会原样落到 `ou` 上（`ou` 首选 区、`ditu` 拼出 的图），所以词级排序（`Engine::word_log_prob`）与整句词图（`SpanWord::reading`）都扣 `sentence::reading_cost` = `-ln(读音占比)`；用户选过的仍按「同输入串选择次数」排在前面。整句按哪种切分转由 `Engine::sentence_segmentation` 定：切分器只按结构排（音节少、完整的在前，打平时贪心），音节数与不完整音节数都与第一种相同的几种（最多 `TIED_SEGMENTATIONS`）各转一遍取路径分最高的（`shangou` 由 shang ou 换成 shan gou → 山沟）。2026-10-22 冻结回放：词 8167 → 8226、整句 2827 → 2788（掉的多是用 `di` 打 的、`du` 打 都 的习惯，和日志里当时选了错句的统计假象）。
- 中英混输的英文词位置：`Engine::set_chinese_first`（配置 `[general] chinese_first`，缺省关）关着时拼音不像话的输入英文排第一（`extras::insert_english`，
  用户老选中文词时仍让中文在前），开着时整句先插、英文词紧随其后排第二（`query_inner` 里两步的先后按开关掉转）；句末英文词并入整句（`EnglishTail`）不受它影响。
  关掉「打拼音时也给英文词」（`english_in_chinese = false`）时 `split_english_tail` 整条停：英文词既不进候选，也不作为句尾参与分词（#766）。
  缺省关是回放定的（9241 词 / 269 条英文上屏：缺省开英文首选 82.5% → 7.1%）。
  条数与最高位次两档：`english_max_in_chinese`（1–4，缺省 4 = 1 条精确词 + 最多 3 条补全；1 只留最靠前的一条）与
  `english_min_position`（1–9，缺省 1 不限）；`extras::place_english` 在整句也插进来之后把英文块挪到第 K 位，没有中文候选时不动。
  双拼 / 注音末尾落单的一键是正常的「还没敲完」，若连同前面的音节正好拼出词库里的整词（`veg` → 这个），整串不算「拼音不像话」：
  英文候选不让到第一、也不给补全（`query::phonetic` 里的 `covered_by_word`，全拼的落单字母另有「可能敲反了」的意思，不适用），
  拼不出整词（`cat`）时英文照旧排第一（#753）。
- 中西文自动空格（`spacing` 判交界，`engine/spacing.rs` 判上文；配置 `[general] auto_space`，缺省关）：`commit` / `take_raw` 自己补前导空格并把它记成一次直通，
  候选内部的交界在 `commit_with` 里补、`LastCommit.chars` 按补后的字数记；直通字符由壳先问 `auto_space_before`。上文取最近一条没退格的上屏末字，
  还没上屏过时用组句首键读到的应用前文（`rescoring_before`），删了一半或焦点离开后不知道上文就不补。
- 自定义短语按权重排（`weight`，1–999，缺省 10，可以重复）：`insert_custom_phrases` 把这个码的短语按权重从高到低（同权重保持列表顺序）
  放在最前，依次标 `Custom(1)`、`Custom(2)`…，`CandidateLayout` 照这个名次固定格子。同码同文本不能重复（`validate_phrases`）。
  0.3 之前的配置写的是 `position`（1–9），读时换成 `11 − position`，设置里保存时 `position` 那一行原地改成 `weight`（`config/phrases.rs::position_to_weight`）。
  英文模式下也插（#763）：以前这里有 `english_mode` 早退，英文短语如 `dont` → `don't` 在 Caps Lock 亮着时不出；
  但只插内容不含汉字的短语（`sentence::is_han` 判），中文短语（含中文模式右键「置顶」按拼音加的那条）在英文模式按空格会直接上屏中文，维护者评审时指出过（`uu` → 文本、`he` → 和）。
  时间变量（#800，`custom_phrase::expand_variables`）在 `insert_custom_phrases` 里按当时的本地时间换掉，候选文字就是上屏文字；`stored_text` 对 `Custom(n)` 按 `scope_phrases` 的名次找回带变量的原文，置顶 / 取消置顶才认得。
- `custom_phrase::merge_replacements` 把平台给的「输入码 → 短语」表（macOS 系统文本替换）并进配置里的自定义短语：每条用缺省权重接在后面，
  输入码不是小写字母、已有同码同文本的跳过；Core 不管数据从哪来。
- 拼写纠错（`correction`）：整段一处编辑的变体里相邻换位允许末尾音节没敲完（`loose_segmentation`，`mignt` → `ming t…`；
  有「每个音节都完整、末尾不是落单单字母」的变体时残尾变体不参与，免得 `keyyi` 的 可以 被 `ke yi y…` 抢走），
  纠正之间比较时换位减 `TypoCosts::correction_transpose_discount`（1 nat，CLI `--tune correction-transpose=`），与原样比不减；
  2026-09-15 回放 283 词 / 23 句：词首选 89.0%、整句 82.6%，与改前持平（折扣若也用在与原样比，`zhongwne` 会误纠成 中文，整句掉一条）。
- 原样优先（#808）：敲的拼音个个音节完整（`engine::reads_as_typed`）时整段纠正不做；整句首选只按原样读，
  条件是原样的最优路径「读得通」（`Engine::credible`：纠正那条没有的词语言模型都认识——没接模型时退回主词库——且至少一个多字词；两条共有的词不算，长句里一个模型不认识的专名（菩提伽耶）不会让整句被纠成 新法案…；
  全靠单字或模型不认识的碎片拼出来的，如 `zanen` 的 赞 + 恩 / 咋嫩，不算）。读得通时敲错边（含个人敲错表的折扣）读出的最优路径只当第二条整句；
  这条读音对不上敲的字母，不进选词重排：试过按首条的单元放它进去，重排里的语言模型分与位置先验只看词常不常用，
  冻结回放里 上屏（shangping）被 商品、这俩（zhelia）被 这里、肃然 被 虽然 顶掉，词首选 −15、整句 −22。
  读不通时照旧噪声信道比。开关 `[general] spelling_correction`（`Engine::set_spelling_correction`，CLI `--no-spelling-correction`，缺省开）：
  关掉后 `find_correction` 直接返回、`expand_positions` 不加敲错边，个人敲错表留着不用。

`EngineSession` 保存可挂起的组句、标点、历史与学习链，`Engine::swap_session` 在同一个引擎里交换输入状态，共用词库与落盘服务。切换上下文时清除查询及异步预测缓存，并由平台恢复各自私密状态。

异步预测按请求序号保存待处理状态：组句请求与翻译请求分别只保留最新一条，可同时等待结果；同类旧请求与清空输入后的结果都会丢弃。

`Engine::raw_preedit()`（`engine/raw/`）只读返回 `RawPreedit { text, cursor_bytes }`：完整未上屏组合及 UTF-8 字节光标，与随后 `take_raw()` 共用文本生成，保留大小写、显式分隔符及光标后的剩余内容，不包含待处理辅码；不运行候选查询、不学习、不记日志、统计、历史或展示回报。
注音继续输出符号；光标按解码单元内按键前缀产生的符号数映射到完整单元的同序字符边界，轻声先敲时采用逻辑位置，一声空格不占显示字符。未知键仍按原解码规则聚到尾串，其光标跟随输出位置而可能不单调；首位固定 0，末位固定完整文本长度。`take_raw()` 的提交和清理顺序不变。

`Engine::discard_input` / `EngineSession::discard_input` 用于隐私能力变化时无痕清理输入，包括透传缓冲、学习链和暂存词汇曝光；`set_private` 只切换写入开关，保留已输入的组句。

语音（`voice/`、`engine/voice.rs`）：`Transcriber` 是云端与本地离线识别的接口（非阻塞，`start` / `feed` 单声道 16 位 PCM / `finish` / `cancel`，`poll` 取 `TranscriptEvent`），现在只有 `NoTranscriber`；系统识别器留在各平台壳里。不论哪种识别方式，结果都经 `Engine::commit_voice` 上屏：繁体输出开着时转繁体，记输入历史、例句、输入日志（`InputSource::Voice`）与输入统计（词数按语言模型切，不算选词、不计键数时长），不记词频。设计见 `docs/design/voice-and-swipe.md`。

滑行（`swipe/`）：`SwipeDecoder` 把轨迹（`SwipePoint`）、字母键几何（`SwipeKey`：中心与宽高）与候选词（`SwipeLexicon`：小写字母串 + log10 词频先验，按首尾字母分 26×26 桶；`from_dictionary` 把主词库各词音节连起来、同串词频相加，`from_word_list` 把英文词表的 Zipf×1000 除回去）解成字母串，按可能性排。`ShapeDecoder` 按理想路径与轨迹的形状差（48 点重采样逐点均方 + 字母按序覆盖的动态规划 + 轨迹偏离最远点 + 起终点，键宽归一）乘 5 再减去先验打分，起终点 1.1 键宽内的桶才比、按「先验 − 起终点差」上限从高到低提前停，回前 5；词表空或起终点附近没有词时退回 `CornerDecoder`（起点、终点、拐点最近的键，拐点是前后各 0.4 键宽一段方向差过 50°、连成一串取最急的）。解出的字母由平台像点按一样逐个进组句。设计见 `docs/design/voice-and-swipe.md`。

## crates/qingjian-translate

`Glossary`，本地 TSV 释义表（词性 + 译文）；`LevelTable`，词汇等级表（`assets/levels/levels-{en,ja}.tsv`，CEFR A1–C2 / JLPT N5–N1，
`uv run tools/corpus/levels.py` 从 `data/levels/` 的原始 CSV 生成，来源与许可见 `assets/levels/README.md`），「统计」页按级数词汇用，不进候选。

## crates/qingjian-learning

- `FrequencyLearner`：用户选择次数（`user.tsv`，带最近使用日）、按输入串记的选择（`user-choices.tsv`，词级排序里同输入串选过的优先；泛化在**记录侧**完成：
  选择除记字面键外，读音键（候选音节连写）上另记一份——`wod` / 双拼选的，换回全拼再打同样认账；单字母简拼（`g`）不泛化，
  那里翻页选词的意图太弱，带进全拼键会反过来压用户后来认真打的（`g` 选的 该 压住 `gai` 下的 改）。查询只看字面键，
  更短的查询（`yiy`）不带走更长输入（`yiyi`）下的选择，隔串压不到别人；英文词位置的仲裁同样只按字面键）、用户词（`user-words.tsv`，主词库同格式，
  Engine 与主词库一起查）、个人英文词（`user-english.tsv`，回车原样上屏的英文词与选过的英文候选，与随包英文词表一起出候选且在前；
  原样上屏的串要像一个词才学：`MIN_ENGLISH_WORD_LETTERS` = 2 到 `MAX_ENGLISH_WORD_LETTERS` = 15 个字母、切不成完整拼音、
  也不能读成「拼音头 + 英文尾」——打不出来时整串上屏的失败拼音学进去会反过来堵住混输那条路，越打不出来学得越多）、
  个人敲错表（`user-typos.tsv`，接受过的 (敲的, 要的) 音节对，词图敲错边与整段纠错的代价按它打折）与个人 n-gram（`user-ngram.tsv`，Core `sentence::UserNgram`，
  二元 + 三元在线计数，整句转换与词级排序里与静态模型插值；Tab 接受的云端整句按 `sentence::segment_text` 切词后也记；
  连着选出的两个词记够次数自动造词进用户词，一段拼音分几次选完的合成词记两次也造）。
  候选右键菜单的降权记在 `user-demoted.tsv`（`frequency_learner/demotions.rs`，输入串 → 词 → 降了几次，封顶 99，不衰减），私密输入与关学习时照记。
  词频、个人英文词与敲错表三张表带**时间衰减**（`frequency_learner/decay.rs`，牛顿冷却）：最近使用日记在
  `user-days.tsv` 等 `*-days.tsv` 侧车文件里（主表保持旧格式两列 / 三列，旧版本读到新文件不丢数据；侧车丢了整份退回文件修改日），
  查询按 `count × 2^(−间隔天数/90)` 折算有效次数。衰减只用在排序权重上，记录本身不清——真要清等冷了一年
  （`COLD_DAYS`，落盘时连侧车一起清）；英文词最低按 1 保留在词表里。半衰期可 `--tune decay-days=` 覆盖，0 为不衰减。
- `InputLog`：输入日志（`input-log.jsonl`，每次上屏一行：敲的键、切分、看到的前几个候选、选了第几个、来源、纠错、撤销，
  Core `InputLogger` trait 的落盘实现，`[general] input_log` 缺省开，只写本机，给离线回归评测与个人模型用）。
- `UsageStats`：输入统计（`usage-hourly.tsv`，按小时记汉字 / 中文词 / 英文词 / 上屏 / 选词 / 首选 / 键数 / 打字毫秒 / 删过，Core `UsageMeter` trait 的实现，
  Engine 每次上屏 `Usage::of_text` + 按来源定词数，整句按 `segment_text` 切词数；键数与打字时长由 Core `Typing` 数：从这段第一键或上一次上屏起，键间停顿超过 5 秒不算。
  壳把输入引到自己的工具框（安卓的快捷翻译原文、常用语编辑框）时 `Engine::set_metered(false)`：这些上屏不记统计（学习照常），键数时长一并清掉免得算到下一次头上。
  **每次上屏当场追加一行**（进程被杀、断电最多丢正在写的半行，读时跳过；末尾是半行时先换行再追加），同一时段多行读时相加；`flush` 只在追加失败过、或文件行数比时段数多 `COMPACT_AFTER` = 2000 时整理重写（原子写）。
  0.2.2 以前按天的 `usage.tsv` 照读不写（不分时段），降级回老版本时老版本只写它，两份不冲突。与输入日志无关，设置「统计」页显示，`book_scale` 折成几本《某书》）。
  派生数统一在 `usage_stats::figures`（re-export 成 `qingjian_learning::figures`）：近 7 天（`RECENT_DAYS` = 7、含今天、滚动，不按周归零；生词本「近 7 天新见」同一个 `recent_start`）、本月、累计、过去 24 小时 `last_24`、速度（`TODAY_MIN_MS` = 30 秒、平均与最快一天 `AVERAGE_MIN_MS` = 5 分钟）、首选率、一次打对、每字键数、最多 / 最快的一天、有输入天数与日均。
  PC 设置程序统计页与安卓 App 都只调这里，不各自再算，以后多端同步按同一口径；PC 年历一列从周日起只是排版，不是统计数。
- `VocabularyBook`：词汇记录 / 生词本（`user-vocab.tsv`，Core `VocabularyTracker` trait 的实现：学习语言的每条译词看到过几轮 / 上屏过 / ⌥+数字 打出过几次，
  后三列是「认识」与最近一次的中文、例句（老文件只有前七列照读）；例句由 Core `ExampleCollector` 攒：上屏带译词的词先挂起，句末标点、换行、`break_chain` 或满 80 字时交出整句；
  时间线另记 `vocab-log.tsv`（`encounter_log`，一次相遇一行 `时间\t语言\t中文\t说法\t读音`，落盘时追加）：Core 上屏带译词且至少两个汉字时 `record_encounter`，整句（一次上屏的，与 `ExampleCollector` 收出的分几次上屏、至少 4 个汉字的一句）经 `GlossFiller::translate` 交给 `qingjian-predict` 的整段翻译线程（攒 5 秒或 10 段一批），`poll_glosses` 取回再记；
  设置程序的生词本页也写这个文件，两边写前 `refresh`：按修改时间与长度看出对方写过，就把对方删掉的词删掉、「认识」取对方的；Core 私密输入统一跳过曝光和提交写入，但仍可读取已有记录用于排序和生词标记；
  Engine `annotate` 据此填 `Sense::fresh`，看到轮次不到 `FRESH_UNTIL` = 3 的译词壳里画橙色；「看到」按上屏那一刻屏幕上那一页算，壳每次画完 `Engine::note_displayed` 告知当前页）。
- `ClipboardHistory`：剪贴板历史（`clipboard.jsonl`，一行一条 `{text, image?, at}`，新的在前）。文字原样存（换行、缩进、制表符都在，所以不用 TSV），
  一条最多 2 万字符；同样的文字或同一张图再复制挪到最前；条数上限可设（`set_limit`，缺省 `DEFAULT_CLIPBOARD_LIMIT` = 1000），超了丢最旧的。
  图片存在 `clipboard-images/`，按内容的 64 位 FNV-1a 取名（同一张只存一份），条目删了、没别的条目用这张图就删文件。增删时整份原子重写——
  1000 条也就几百 KB，毫秒级；以后要全文搜索或上万条再换 SQLite，壳只经这个类型读写。壳只负责读系统剪贴板、把文字或图片交进来。
- `PhraseBook`：常用语（`phrases.json`，`[{text, group, code}]`，靠前的常用）。增删改、移到最前（`move_to_top`）、分组（`groups`，「默认」总在第一）；
  输入码只留字母、转小写、最多 32 个（`MAX_CODE_LEN`，与 Core 自定义短语一致；以前存下的超长码转换时跳过），`custom_phrases` 把带码的换成 Core `CustomPhrase`，权重都是缺省值，同一码按先后排。
- 各表落盘走 Core `storage::write_atomic`（临时文件 + fsync + 改名），加载按行容错（坏行警告跳过，真读不了壳退回内存学习），
  壳激活期间每 60 秒 `Engine::flush_learning`；IMK 回调边界 `imk::catch_panic` 拦 panic、缓冲区字母原样上屏（见 architecture.md「崩溃不丢」）。

## crates/qingjian-predict

请求参数不按模型名区分：收到 400 / 422 且明确拒绝当前请求里的可选参数时，省略被拒的 `temperature`、`reasoning_effort` 或 `thinking`；`max_tokens` 被拒则改用 `max_completion_tokens`，在调用方上限上预留有限的 4096 token 供推理。替代的 token 字段也被拒时直接返回错误，不去掉生成上限。能力以客户端为单位记忆，后续请求直接使用兼容写法；每次重试都要实际改变请求，认证、限流、服务端错误与含糊的校验失败不触发参数降级。测试连接、联想、释义与翻译共用这条路径。

TLS 由 feature 选：缺省 `native-tls`（系统 TLS，桌面各壳与 cli 都用这个）；`rustls`（关掉缺省时）走 ring + `webpki-roots` 内置的 Mozilla 根证书、`tls_backend_preconfigured` 交给 reqwest，不用平台证书库（`rustls-platform-verifier` 在安卓要 Java 侧组件），安卓 `apps/android/core` 直接按路径引、关缺省开 `rustls`。工作区的 `async-openai` 不带 TLS feature，直接用它的（`tools/gloss-gen`）自己开 `native-tls`。

API 用量由 ChatClient 的单次发送入口计入：每次 HTTP 尝试分别累计，关思考参数与回复格式的兼容重试都按实际次数计；SDK 隐式重试关闭。CloudPredictor、CloudGlossFiller、ConnectionTest 的 `*_with_usage` 接口可指定共享统计文件，旧构造入口保持无统计行为。`api-usage.json` 只保存计数，文件锁合并跨线程／进程更新，原子写入；损坏文件保留并报告错误。缺失 usage 记未知，不估算 Token。
每个有结果的请求还在同一把锁里往旁边的 `api-usage-hourly.tsv` 追加一行（日期、时、请求、失败、输入、输出、合计），超过 256 KB 整理成一个时段一行，统计页画按天、按小时的图用（`ApiUsageHours`）。

请求控制：默认防抖 300 ms、最短起始间隔 3000 ms。Engine 的自动入口尊重 `automatic`，显式入口 `request_manual_prediction` 跳过防抖但遵守间隔；取消通道消息清除待发输入，已发结果由序号过滤。缓存命中不受网络间隔限制。SDK 隐式重试关闭，保留模型拒绝关思考参数时的一次显式兼容重试，以及 401/402/403 暂停 10 分钟。回复格式（`reply/`）按 `json_object` → `json_schema` → 不约束逐级降：报错提到 `response_format` 就降一级重发，客户端记住这一级；`json_schema` 用各调用方给的 `ReplySchema`（联想 / 问字 / 翻译在 `prompt.rs`，释义与整段翻译在 `gloss/`，测试连接在 `connection/test.rs`），改了提示词里的输出格式要同步改它；正文统一剥掉 Markdown 代码块围栏。
候选菜单的模型译文重生成是显式翻译请求（`CandidateTranslation`）：目标语言为当前学习语言，总走模型联想的提供方（不走选区翻译的独立提供方）；worker 给它单独留一条「最新一条」槽位，和组句联想、整句翻译一起发出，互不覆盖。
翻译请求（`PredictionKind::Translate`）出错或撞上暂停时 worker 回一条空 `Prediction`，壳当场结束「翻译中」、报失败，不干等超时；联想照旧不回。回包解析前去掉 `service_tier`：async-openai 那边是封闭枚举，OpenRouter 回 `"asap"` 会让整条解析失败。

Windows 的 `Router::recompose` 在输入清空时调用 `Engine::cancel_prediction`，同时撤销待发联想与译文；不能只作废各类结果的序号，否则 worker 仍会发送并占用请求间隔。输入未清空时保留按类作废，使未变化的整句候选译文可以继续返回。

- `CloudPredictor`：`Predictor` trait 的网络实现（async-openai，OpenAI 兼容接口，默认 DeepSeek），后台线程按种类合并待发请求，组句与翻译并发发送；`submit` / `poll` 非阻塞。
  `PredictConfig` 是配置的 `[predict]` 分节。只在组句中联想，一次请求给云端词（容错校验后补进候选第一页末尾 `[predict] slots` 格，缺省 2，不预留不占位，
  前面的本地候选不挪；排布在 Core `CandidateLayout`）和整句补全（preedit 右侧，Tab）；上屏后不联想，本地历史不进请求。
  简拼（半数以上音节是缩写）的请求 `max_items = 0`，只求整句补全（`prediction::mostly_abbreviated`）：按声母凑出来的词大多是生造词，
  拼音校验又按首字母序列匹配放行缩写，拦不住；问字模式的答案不受这条限制。
  九宫格的作用域是数字，`submit_prediction` 改用同一段的 `QuerySnapshot.pinyin`（九宫格时是拼音行 `typed_display`），没查过这段不发；所以要先 `query` 再 `request_prediction`。
- `CloudGlossFiller`：释义兜底（Core `GlossFiller` trait，与 Predictor 分开的线程与通道，攒 1.5 秒 / 8 个词发一次，问过不再问）：
  随包释义表没有的词库词 / 云端词上屏后入队，结果壳每秒 `Engine::poll_glosses` 经 `Translator::learn` 写进 `qingjian-translate::PersonalGlossary`
  （`user-glossary-<语言>.tsv`，`LayeredTranslator` 个人表优先）；随云联想开关一起开。
  同一个 Filler 另起一条整段翻译线程（`gloss/phrase/worker.rs`，提示词 `phrase/prompt.rs`，攒 5 秒 / 10 段，本进程内同一段不重复翻）：生词本时间线用，整句等没有现成译词的上屏中文经 `GlossFiller::translate` 入队，`poll_translations` 取回 `Encounter`。
  云释义的系统 prompt 在 `gloss/prompt.rs` 按学习语言分派：德语臂 `GERMAN_SYSTEM_PROMPT` 要求名词带冠词 der/die/das 且首字母大写、动词用不定式；
  返回译文经 `clean_text` 按语言设限，德语臂上限 40 字符（`MAX_GERMAN_CHARS`，西语臂 32），字符白名单 `is_latin_ext_char` 为西 / 德共用
  （拉丁扩展区间 0x00C0–0x024F，汉字与西里尔字母不收）。
- 问字键（缺省 `u`）开头是问字模式（`PredictionKind::Question`，答案带读音、不校验拼音），`?` 开头要 `ModeKeys::question_mark` 开着才算（配置 `[shortcut] question_mark`，缺省关，壳用 `Engine::takes_question_mark` 决定空缓冲区的 `?` 是入口还是标点）；问字 / 表达式两种前缀模式共用 `Engine::english_word_candidate`：整段命中英文词表（用户表优先）时本地给英文词候选、不发云请求，于是 u 开头英文词（`unchanged`）不被问字吞掉；`PredictionKind::Translate` 是壳里快捷键触发的「翻译选中文字」
  （双向：汉字为主译成学习语言，外文译回中文，`prediction::translation_target`），译文走结果的 `sentence`。

## crates/qingjian-voice

云端语音识别，实现 Core 的 `Transcriber`。现在只有火山引擎豆包流式语音识别（`volcengine/`）：`VolcengineTranscriber` 每段识别开一个后台线程（`stream.rs`），壳那边 `feed` / `finish` 经通道交过去、`poll` 从通道取，`cancel` 置标志让线程自己收尾。
线程里是阻塞的 tungstenite WebSocket 跑在 rustls 上（`tls.rs`，ring + `webpki-roots`，不开 tungstenite 自己的 TLS feature，加密实现显式给，不用装进程级缺省），握手后把读超时设成 20 ms，读不到就回头看有没有新录音；写不设超时。
帧格式在 `protocol.rs`（有单元测试）：4 字节头、序号、长度、gzip 载荷，最后一包序号取负；`X-Api-Sequence: -1` 是文档要求的固定头。中间结果只在文字变了时交。
`test_connection` 识别半秒静音、阻塞到有结果，设置页「测试连接」用。错误文案：`#[error]` 英文给日志，`VoiceError::describe` 中文给用户。
rustls 与 `webpki-roots` 提到了工作区依赖，与 `qingjian-predict` 的 `rustls` feature 共用一份。

## crates/qingjian-format

`.qj` 数据容器（`Container` mmap 读、`Writer` 写、`Table<T>` / `Text` 零拷贝视图、`hash` 可落盘哈希索引、`Metadata` 名称 / 许可证 / 署名）。
词库与语言模型都能 `write_qj` / 从 `.qj` 打开，启动 50 ms；`cargo run --release -p qingjian-dict-convert -- pack dict|lm --name … --license …`
生成 `data/generated/{dict,lm}.qj`，`bundle.sh` 在 TSV 更新时自动重打并只把 `.qj` 打进包。设计见 `docs/design/architecture.md`「数据文件：`.qj` 容器」。

## crates/qingjian-neural

candle 加载 Transformer（GPT-2 风格 decoder，导出成
`model.safetensors` + `config.json` + `vocab.json` 三件套）。features `accelerate` / `metal` 换后端，壳用 `metal`。
Metal 缺少 `MTLResidencySetDescriptor`（如 macOS 14）时改用 CPU/F32，避免 candle 创建设备时 panic；支持该类时仍用 Metal/F16。
CPU 上有 AVX2 + FMA 时 `CharLm` 把各层线性层与输出层量化成 int8（`hanzhang/char_lm/q8_matrix.rs`，32 个一块各带缩放，与 Q8_0 同一种分块，输入留 f32），
词嵌入留文件里的 f16、查完转 f32；没有这两条指令集的机器、设了 `QINGJIAN_NEURAL_DTYPE` 时照旧 f32。
通变常驻 +112 → +36 MB，成绩不变、查询耗时持平。没用 candle 的 `QMatMul`：它的 x86 SIMD 按编译期 `target_feature` 选，
缺省目标只有 SSE2，发行构建走标量（150 句平均 161 → 386 ms）；整个程序开 AVX2 又会让没有 AVX 的赛扬 / 奔腾银牌机器起不来，所以自己写一个运行时探测的。
`CharLm` 的公开方法都在一个最多 4 线程的专用 rayon 池里跑（`hanzhang/char_lm/threads.rs`）：用全局池时 20 线程的机器上通变打字时 Server 平均占整机 26%、最高 54%，
多出来的几乎全是线程空转；限 4 线程后生成平均 163 → 127 ms、开发集 500 句查询平均 133 → 95 ms，CPU 时间降到约四分之一，成绩不变（2026-10-28）。
知意的 ONNX Runtime 本来就是 2 线程（占整机 8%）；关掉它的空转（`intra_op.allow_spinning`）只省 0.1 个核、查询却慢 5 ms，没关。

feature `zhiyi` 是知意的选词模型（`zhiyi::ChoiceScorer`，实现 Core 的 `ChoiceModel`）：分词、对齐、序列拼法共用，前向两个实现——
safetensors 版由 candle 跑（mac），feature `zhiyi-onnx` 加上 int8 ONNX 版由 ONNX Runtime 跑（Windows，CPU 两线程）。`ort` 用 `load-dynamic`，编译时不链接，
运行时按路径加载 `onnxruntime.dll` 1.24.4（`api-24`），由 `apps/windows/installer/fetch-onnxruntime.ps1` 从 NuGet 取、按包的 SHA-256 核对。
DirectML 试过不用：每次打分的序列长度都不同，Radeon 880M 集显上均衡档中位 200 ms、准确档 182 ms，同机 CPU 两线程 28 / 37 ms、省电档 18 ms（2026-10-21）。
两版对拍：`zhiyi::tests::onnx_agrees_with_candle`（设三个环境变量才跑，见测试注释）。
分词是自己的 `zhiyi::bpe`（只编码）而不是 `tokenizers`：25.6 万词、58 万条合并规则的 `tokenizer.json` 用 `tokenizers` 读要常驻 65 MB、
加载时峰值多 300 MB，自己只留编码要的两张表，知意常驻 −35 MB、加载峰值 656 → 384 MB（CLI 实测）。
与 `tokenizers` 逐 token 对拍：`zhiyi::tests::tokenizes_like_the_reference_tokenizer`（仓库 `docs/` 每篇文档的每一行与全文，设知意模型目录才跑）。
ONNX 版另有 composition 头 `char_logits`（每个拼音槽位一行、6163 字的 logits，列与 `unified_vocab.json` 的 `chars` 对应），
`ChoiceScorer::compose` 取它、逐行 log softmax，接到 Core 的 `ChoiceModel::compose`；candle 版没接这个头，返回 `None`。
它对每个音节各自出分布，字与字之间不管连贯（单用它逐位取最好：`yanxiashigong` → 眼下是公），所以只当整句词图里的字分数用，成词交给词图。

Core 侧（`engine/choice`）：整句转换前按（前文, 音节串）取这张字分布表（`Engine::char_table`），没有就记下、和选词打分一起在停顿后送后台，
回来后重查（与选词重排同一条 `request_rescoring` / `poll_rescoring` 流程，最近 8 张缓存）。停键后要过两轮后台（出字中位约 15 ms，重查后再选词打分约 23 ms），
两个后台线程交了结果都调壳给的 `Engine::set_rescoring_wake` 回调（`rescoring/wake.rs`），Windows Server 拿它往工人循环投 `Work::Wake`，不等 20 ms 一拍的轮询；
加上防抖从 80 ms 缩到 30 ms，按 social 开发集 300 句实测的各段耗时推算，停键到首选换好中位 154 → 84 ms、120 ms 内换好的 3% → 93%（不含 IPC 与重画）。
词图里每个词加
`EMISSION_WEIGHT`（0.5）×Σ（该字 log 概率 − 这个槽位最好的字，下限 `EMISSION_FLOOR` −12）；读音被敲错边、模糊音改过的字
字分布说明不了，固定 `ALTERED_EMISSION`（−4），取舍交给原来的纠错代价（直接用绝对 log 概率时回放整句 82.6% → 79.7%，纠错路径被整条压掉）。
2026-10-23 冷启动整句评测（知意 10-21 版、均衡档，基线 → 接上）：自己的输入 58.6% → 63.5%、本仓库文档抽样 29.7% → 40.8%、
维基新闻留出集 59.5% → 69.0%；去掉上文后提升缩到 +1.8～3（模型很靠上文）；冻结回放词 89.0% → 89.7%、整句 82.6% → 80.5%
（掉的多是日志里当时选了错句，与 `di` / `du` 那次同类的统计假象，另有「在 / 再」口语写法）。

Core 的 `sentence::SentenceScorer` 有两个实现，同一个 trait 拿到**两种条件**（`context` 光标前文、`keys` 这批路径共同解释的那段按键），各挑自己训练时的那个、忽略另一个：

- `CharScorer`（含章·知微，字级模型）用 `context`：给「前文 + 整句」按字累加 log 概率，前文的每层 K / V 缓存（`PrefixCache`），
  同一段前文只算一次，每个候选只算自己那几个字（64 字前文 × 8 条 28 ms，Metal）。
- `P2cScorer`（含章·通变，P2C 模型）用 `keys`，不看前文；它 `generates()` 为真，是**生成器**：不拿它重排词图路径，
  整句以它直接生成的为主（见下面「生成器」一段）。早先当重排器用时冻结集 8322 句上 42.49% 对字级模型的 41.80%，
  2026-10-27 social 开发集上重排 57.1%，改成生成为主 59.1%、前三 61.3 → 75.2%。
  字级模型没有下岗，它是移动端「光标联想」那类 `P(下一段 | 前文)` 功能的基础——P2C 的 `<eos>` 明确切断上文，结构上做不了那件事。

**生成器**（`Engine::generates_sentences`，`engine/query/generating.rs`）：拼音干净（没有残缺音节、切分覆盖全部、没纠正）且至少
`GENERATED_LEAD_MIN_SYLLABLES` = 6 个音节时，生成的整句排在词图的整句前面；更短的、只有末尾一个音节没打完的、纠正过的，词图第一、生成的第二
（短输入里词图带着个人习惯，生成器会把 版本好 / 不一致 换成 坂本号 / 不移植：冻结回放按字数拆，2～5 字新错 289 新对 30；没打完、敲错的拼音生成器读不准）；
切分读不通整段（中英混输）时照旧生成的在前。生成句与词图整句同文时用词图那条（带读音，上屏能学）、排在生成给的名次上；
生成 + 词图整句合计最多 `SENTENCE_CANDIDATES` 条（#769）。没接生成器时只在读不通整段时生成。

Engine 侧在 `engine/rescoring/`：接了重排用的打分器（知微）就取 Viterbi 前 `RESCORE_PATHS` = 6 条路径按 `路径分 + λ·(神经分 − 静态二元分)` 重排（λ `NEURAL_WEIGHT` 0.5，
个人 n-gram / 用户加分 / 代价不动）。**神经分只决定名次，不写回 `Conversion::score`**：P2C 打的是 `log P(汉字 | 拼音)`，
比静态分高十几二十 nat 且抬升随句长变化，写回去会污染跨读法的比较（拼写纠错的原样 vs 纠正、混输的头段 vs 整段），
那两处的门槛都按静态尺度定；按贝叶斯展开也只在同一串按键下可比。分走「(前文, 按键) + 文本 → 神经分」缓存 `NeuralCache`（两个条件任一变了整张作废）；
按键取的是**当前这批 patterns 覆盖的字母**而不是整个作用域——英文尾巴那条只转换 head，拿整段当条件会让它凭空背上没覆盖的字母；
同步打分器（`with_sentence_scorer`，CLI 评测）当场补分，
异步的（`with_async_sentence_scorer`，后台线程 `RescoreWorker`）查询不等模型：缺分的记下来，壳停键后 `request_rescoring`、`poll_rescoring` 到了再 `query` 一次；`poll_rescoring` 只在最新那次请求的结果到了才返回 `true`（每次请求带序号，旧请求的结果照样进缓存），壳据此停轮询。
前文优先用壳给的应用光标前文（`set_rescoring_context`），没有用当前应用最近 64 个上屏字符：`InputHistory` 按 `set_application` 的应用分开记（每个 256 字、最多 16 个应用），`[model] context_memory_seconds`（缺省 60，0 不过期）没上屏就过期，mac `activateServer` 调 `enter_input_box` 清当前应用的，Linux 按 `EngineSession` 每个输入上下文一份；Windows 还没有「换了输入框」的信号。CLI `--neural <导出目录>`（`--neural-weight` / `--neural-context` / `--neural-async`）。

P2C 教师强制打分 `CharScorer::score_p2c(keys, texts)`：前缀 `<eos> + 完整拼音 + <sep>`，只累加候选字符的 log 概率（不计 EOS），
共享拼音前缀 KV，候选批量前向；超过上下文时报错，不截断拼音。CLI 实验入口 `--eval-p2c data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm --eval-text data/eval/sentences.tsv`，
用 `--neural-weight` 调 λ。评测适配器与产品端 `P2cScorer` 同一条件，只是推理出错时直接终止（不静默回退到基线冒充成功）。
`--eval-details <输出.jsonl>` 为任意整句评测保存逐句首选、整句候选、全部候选与查询耗时，支持配对比较，仍不上屏、不学习。
没命中首选的句子另记 `loss`（正确答案丢在哪一步）与 `path_rank`（在词图前 64 条整句里排第几），报告末尾按 `loss` 计数：
`Engine::diagnose_sentence` 把原句强制对齐到同一张词图（`sentence::diagnose`，同样的格子与打分，只是不剪枝、不限束宽），分成
`unreachable`（词库拼不出）、`segmentation`（换一种切分才拼得出）、`pruned`（词在格子里排在 `SPAN_CANDIDATES` 之外）、
`search`（分数更高却被束搜索挤掉）、`scoring`（在词图里但分数低；`loss_detail` 里有两条路径的总分、语言模型分与出字分）、`assembly`（词图对了、候选组装排错）。
2026-10-24 开发集上 `scoring` 占 70～92%、`pruned` 6～25%（多是单字），其余各约 1～3%；每格放宽到 12 个剪枝几乎消失但首选不变。
格子按词频留 `SPAN_CANDIDATES`（6）个之后，再往后看 `EMITTED_SPAN_POOL`（24）个，出字分比留下的都高的至多收 `EMITTED_SPAN_EXTRAS`（3）个（`sentence::kept_hits`，
格子缓存存整个备选池、每次查询现挑，因为出字分随上文变；没有模型时出字分恒 0、一个不收）。与一律放宽不同，收进来的是模型看好的那几个：
2026-10-26 social dev 首选 74.1 → 74.5%（改对 38 改错 16）、首页 +0.7，`pruned` 243 → 162，查询多约 1.5 ms；诊断按同一规则判是否在格子里。

P2C 自由生成实验：`--eval-text <集> --eval-generate data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm`（`P2c::convert` 束宽 5，不经词图不经重排），
量的是重排这层架构的上限代价；2081 句上首选 51.9% / 前五 73.5%，对比重排路径的 42.3% / 45.5%，两条路互补（只有生成对 16.5%、只有重排对 6.9%），
代价是 `O(生成字数)` 次串行前向。数字与取舍见 [neural-rescoring.md](neural-rescoring.md)。

**模型直接生成的整句**（`engine/query/generating.rs`，候选类型 `CandidateKind::Generated`）：词图只会把整段按键读成拼音，
中英混输（`yongdockerbushuhenfangbian`）与生词在它那里根本没有路径，出来的只能是把英文段硬读成拼音的废话（用的哦乘客仍不熟很方便）。
词图读不通整段时改问 `SentenceScorer::generate`（P2C 走 `P2c::convert`，字级模型返回空），生成的整句插在词图那几条前面。
「读不通」三条：最优切分里有不完整音节、切分没覆盖到末尾（`woyongvscodexiedaima` 的 `v` 起不了音节）、拼写纠错生效
（`womaileyigeiphone` 的 `phone` 被当成敲错的 `paone`）。拼音干净的输入一条都不触发，常态零成本；
异步打分器下走同一次 `request_rescoring`（`Job.generate`），按键回调不等它。
`GENERATE_BEAM` = 5、`GENERATE_MAX_CHARS` = 32、`GENERATED_CANDIDATES` = 2、`MIN_GENERATED_LETTERS` = 6；双拼 / 注音不走（按键不是模型见过的字母）。
生成的候选没有音节对齐，上屏吃掉整段作用域、不记学习。漏的是「英文词本身就是合法拼音」那种（`zhegeapihenhaoyong` 的 `api` 读成 `a pi`），
光看切分分不出来。

整句候选不止一条：`SENTENCE_CANDIDATES` = 3 条，重排后的前几条路径都进候选表，四个音节
（`ALTERNATE_MIN_SYLLABLES`）以下不给备选（那几格留给词级候选）。冻结集上前三 42.4% → 45.5%，无模型时 38.1% → 43.2%。
`Engine::sentence_paths` 的 `want` 参数控制算几条：拼写纠错对每个纠正候选都要转一次，那条路仍然只算一条。
接了选词模型时长输入改要 `CHOICE_SENTENCE_CANDIDATES` = 16 条，由知意选词头带上文整句重排，重排后只显示前 3 条（`trim_sentence_alternates`，第一页仍留给词）。
要的条数超过束宽时 Viterbi 每个词留前 `NBEST_HISTORIES` = 4 条来路、束宽同比放大：只留一条时末尾几条路径只差最后一个词，束宽 8 放到 32 正确句进前 32 条的比例也几乎不变。
开发集（均衡档）正确句进第一页 devA 67.8% → 74.8%、项目文档 51.7% → 61.2%、新闻 81.0% → 90.1%，首选 +0～2.6；正确句已在候选里时知意只选中 70～84%，瓶颈在重排。
冻结回放整句 78.6% → 78.4%、词 90.3% → 90.2%（持平），CLI 查询多 12～16 ms。
13 个音节以上（`MIN_SPLICED_SYLLABLES`）还拼接备选（`sentence/viterbi/splice.rs`）：沿最优路径每个词界取至多 3 个别的节点、接上最优路径的后半段，
接口后两个词的转移按新上下文重算（得分精确）；再按改动的字排开（`diverse.rs`），与前面重叠的同一处至多两条在前，其余挪后。
末尾节点回溯出的 n-best 在长句上几乎只差句尾（啊 / 阿 / 吖），均衡档重排窗口只有 9 条，句中拿不准的字没有备选。
短句不拼：知意选词头是整组打分，拼出的备选除一处外都与最优路径同字，等于给最优路径多投票，全长都拼时无上文首选 −0.6。
社区开发集首选 74.5% → 74.7%、首页 +0.2，无上文首页 +0.16、首选持平，冻结回放不变。
重排时整句候选的语言模型分与位置先验各乘 `SENTENCE_TERMS_SCALE` = 0.5（`engine/choice`）：整句的先后本就是词图按语言模型排的，两项照算等于数三遍，
挑错的里三到四成是知意对、被这两项压下（维基 → 危机）。`Engine::last_choice_scores` 记这页每条候选的三项分，CLI `--eval-details` 写进 `choice` 字段，
离线就能模拟不同权重；两项各 0.25～1 扫过，0.5 最好。开发集 devC 78.8% → 80.3%、devCnc 70.4% → 73.0%、项目文档 48.8% → 49.7%，
冻结回放整句 78.4% → 79.4%、词 90.2% → 90.1%。剩下挑错的约 3/4 是知意自己偏向错句（事件 / 时间、试验 / 实验）。

冷启动字词实验：`--eval-cold <样本.jsonl> --cold-output <新结果.jsonl>` 强制用缺省配置与内存学习器，
与 `--config` / `--user-dict` / `--predict` / `--replay` 等冲突，不加载或删除个人数据。
每行样本字段 `id/text/keys/source/category`；按完整拼音查候选，不给上文、不上屏，解析失败也记录在分母中。
领域词库用 `--extra-dict` 显式指定，`--neural` 可测字级重排；`--cold-model` 可同时测 P2C beam 5、固定上限 16 字的生成。
实验合并策略保留原首选，将生成的新增汉字候选插在其后，再接原候选；保存全部候选、词库文本/读音覆盖及耗时。
该策略只量候选覆盖和排序代价，没有改产品行为。生成搜索不读取目标字数，候选输出拒绝覆盖已有文件。

## crates/qingjian-lm

`BigramModel`，Core `sentence::LanguageModel` trait 的实现，从 `data/generated/lm.qj`（或 `lm-unigram.tsv` / `lm-bigram.tsv`）加载
（没有这两个文件就退化为一元词频整句）。数据由 `tools/corpus/parquet_to_text.py`（uv 脚本，HF parquet → 简体纯文本）加
`cargo run --release -p qingjian-dict-convert -- bigram --phrases assets/lexicon/phrases.tsv --phrases assets/lexicon/domain_words.tsv --brand assets/lexicon/brand.tsv --brand assets/lexicon/mixed_words.tsv data/corpus/*.txt` 生成；语料在 `data/corpus/`（gitignore）。
短语层不当 token 统计（分词时摘掉、统计完按成分合成一元 / 二元，短语得分等于原来两个词的路径，见 `bigram.rs` 模块注释），品牌词按给定次数写进一元与句首二元。

`TrigramModel`（`trigram/`）：读知意输入法的词级三元模型 `zh.ngram`（`ZYNGRAM1` 格式，ARPA 回退、log10 量化成 i16，整文件 mmap），
实现 Core 的 `sentence::PhraseModel`，选词重排里给整段候选打 ln P(候选 | 上文)（上文取最后一行末 12 字，按字束搜索切词，束宽 8、词最长 8 字），
代替二元模型逐词累加的那一项（`engine/choice` 的 `PHRASE_WEIGHT`）；
也实现 `LanguageModel`（`log_prob_trigram` 带前前词），由 `MixedModel` 按概率与二元模型插值后当整句词图的语言模型（三元权重 `TRIGRAM_WEIGHT`，
一边不认识的词只用另一边）。只用三元时无上文的新闻掉分（不认识的专名、技术词被拆错），插值后六个开发集都涨：devA 63.5→65.6、项目文档 40.8→46.2、新闻 74.1→76.6。`TRIGRAM_WEIGHT` = 0.5：0.3 时冻结回放整句略好（79.1%）但开发集合计略低，0.7 时新闻掉分。读取器按文件头注释的格式自己实现（知意的代码是 GPL-3.0-only，不搬）。
模型 79 MB，不进仓库也不随包，放 `data/models/zhiyi-ngram/zh.ngram`；许可 LGPL-2.1-or-later（底子是有墨输入法 / libime 的模型，混了 FineWeb-2、
Common Crawl、CSL、SkyPile、C4 的统计），可按 GPL-3.0-or-later 使用；再分发前要按知意的 NOTICE 列全来源并跟作者确认。CLI 用 `--ngram`，Windows Server 读用户目录的 `models/zhiyi-ngram/zh.ngram`。
`TrigramModel::vocabulary` 列词表与一元概率，`qingjian_dictionary::supplement` 拿它补出词库没收的词（摸鱼、老梗、七神）：已有的、能切成已有多字词的（进行分析）、非汉字的不补；读音按参照词库里每个字的读音统计猜（词里的读音权重 ×4），次要读音占比 ≥ 0.25 另收、组合至多 4 种；词频按与主词库重合的词拟 ln 词频 = a + b·log10 P（实测约 9.6 万条）。结果挂在 `Engine::set_model_words`（补充词库，单独一格，附加词库重载不动它）。只在本机生成，不分发派生数据：Windows Server 缓存到模型旁的 `zh.words.qj`（`qingjian_platform::model_words`，戳 = 规则版本 + 模型与主词库的大小 / 修改时间，第一次约 1.1 s），CLI 每次在内存里生成（`--no-ngram-words` 关掉）。社区开发集首选 74.7% → 75.0%、首页 +0.43，无上文首页 +0.43，冻结回放整句 +3；CLI 查询 95 分位 209 → 242 ms（中位 +3 ms）。B 站游戏黑话（蜂医、冥古龙）模型里也没有，补不到。

## crates/qingjian-platform

上屏键 `[shortcut] commit_candidate` / `commit_raw`（`config::commit_key`）：`CommitKey` 只收空格、回车（可带 Shift）与一个符号键——Windows 的 TSF DLL 组句时只把不带 Ctrl / Alt / Win 的键送进 Server；
`CommitKeys::route` 给 Windows 与 Linux 的 Router 共用，缺省的那个键返回 `Keep` 走原路径（直输段、只剩一个 `?` 的细节都在那里），换过的键直接上屏，让出去又没配的空格 / 回车组句中吃掉；注音不经过它，macOS 还没接。
设置的导出导入（`config::transfer`，设置程序「高级」页用）：导出去掉 `[[custom_phrases]]`、`predict.api_key`、`status_bar.x/y`，导入时这几项保留本机原值，合并前后都当 `Config` 读一遍；短语的导入导出在 `config::phrases`，导入不收整份配置、读不成 TOML 时按 Rime `custom_phrase.txt` 的 TSV 读。


输入方案是**两条独立的轴**：`Scheme`（拼音侧：全拼 / 双拼九套 / 用户双拼 `Scheme::Custom` / 大千注音 / 关，`[general] scheme`；
认不出的写法能当方案 id 就是 `Custom`，id 由 `GeneralConfig::custom_shuangpin` 给，找不到时壳按全拼跑）
与 `[general] code`（形码侧：`off` 为关 / `wubi86` / 用户方案 id，`GeneralConfig::code`；没写时认旧键 `wubi = "wubi86"`）。两边都开就是**混输**（`GeneralConfig::mixed`）。
`scheme_label(pinyin, shuangpin_label, code_label)` 是状态条显示的方案名（形码在前），做成自由函数；用户方案的名字要读 `scheme.toml` 才知道，
所以由壳手里的 `SchemeSlot` 存着，壳调 `SchemeSlot::scheme_label` / `scheme_name`。

`code_schemes`：形码方案的发现。随包与用户的方案同一种格式：`schemes/<id>/` 下 `scheme.toml`（必须有，`type` 空着或 `code`）+ 码表（缺省 `table.tsv`），
自动上屏与顶字缺省开；`SchemeMeta`（author / version / description / license）两种方案共用，留给分享与设置里显示。
用户在设置里改的规则存配置的 `[code.<id>]`（`CodeRules`，`Config::code_rules`），盖过 `scheme.toml`：随包方案只读，换码表也不丢；
`Config::set_value` 的分节写成 `code.wubi86` 就是嵌套表（外层隐式）。随包的 `schemes` 目录由壳给（macOS `Resources/schemes/`、Windows `assets/schemes/`），用户的在数据目录；
同 id 的用户方案盖过随包的，老位置 `wubi/wubi86.tsv` 只换随包五笔的码表。
`shuangpin_schemes`：用户双拼方案的发现，与形码共用 `schemes/` 目录与 id 规则，`scheme.toml` 写 `type = "shuangpin"` 的才算（形码那边见到它跳过）；
`base` 拿内置方案当底子，`[initials]` / `[finals]` / `[zero]` 按键（零声母按音节）整条替换。内置九套仍编在 Core 里：回放与 CLI 不依赖外部文件，日志里的 `xiaohe` 永远是同一套键位。
`SchemeSlot`（`scheme/slot.rs`）是壳持有的「装着哪套」，两侧一起装：每次配置热加载都按配置重新找方案，但形码方案（含规则）与码表文件的修改时间、长度都没变时不重读码表——
热加载多半是改了别的设置，8.9 万条每次重读是白花的；只改了 `[code.<id>]` 的规则走 `Engine::set_code_rules`，也不重读。
双拼键位表很小，每次重读、变了才换进引擎。`SchemeSlot::apply` 直接收 `&Config`，三条输入（拼音侧、形码 id、规则）都从配置取。
id 不能撞拼音侧的写法与日志里的 `wubi`（`is_valid_id`），因为输入日志把两侧拼成 `<拼音>+<形码键>`，86 五笔的日志键沿用 `wubi`（`CodeSchemeEntry::log_key`）。
目录坏了、码表不在的方案跳过并记日志；选了却找不到的方案壳里警告并按拼音跑。

演进：2026-09-16 之前是 `shuangpin` + `zhuyin` 两个键表达同一个维度，先并成单选的 `scheme`，
同一天又拆成两条轴（单选的 `scheme` 表达不了混输）。旧键（`shuangpin` / `zhuyin` / `scheme = "wubi86"`）
都还在读、文件不自动改写。


`Config`（TOML 配置文件，`[general]` / `[shortcut]` / `[fuzzy]` / `[dictionaries]` / `[apps]` / `[predict]` 分节，首次运行写模板，
`set_value` 用 toml_edit 原地改键保留注释；`[model] enabled` 本地整句模型开关，`LocalModelConfig`；
中英模式两项：`[shortcut] switch_mode`（`SwitchKeys`：勾选 shift / control / ctrl+alt+space，可多选，老配置的单个字符串照读）与 `[general] english_mode`（内置英文模式总开关））；
`extra_dictionaries` 列出 / 加载随包领域词库与用户 `dicts/`；随包的 `general` 例外：缺省加载、列进 `[dictionaries] disabled` 才关，标成补充词库
（mac 壳与 Windows Server 共用，同名 `.qj` 优先于 `.tsv`）；`code_tables` 同构地列出 / 加载随包根 `codes/` 与用户 `codes/` 的码表
（`[aux_code] disabled` 是黑名单，`[general] aux_code_key` 缺省 `;` 且校验后退回缺省、`aux_code_show` 是显示码开关）；
`aux_schemes` 找 `schemes/` 里 `type = "aux"` 的辅码方案（码表 + `[words]` 词码规则），`code_tables::AuxCodeSources::options` 把 `codes/` 与方案按先后列出，
随包 `codes/stroke.qj`（`code_tables::STROKE`）装上时挂笔画的词码规则（与 `dict-convert` 打包同一条），表里只有基础词库的词，自造词与导入词库的词靠它现算；
`load` 按 `AuxCodeConfig::pick`（`[aux_code] scheme`，没选或找不到时第一套不在老 `disabled` 里的）只装一套——设置程序的下拉框用同一个 `pick`，显示的就是输入法实际用的那套，
`snapshot` 只 stat 用户 `codes/` 与 `schemes/*/` 下的文件、不解析 `scheme.toml`，两个壳轮询它发现导入与改方案；
`protocol` 模块是 Windows Server ↔ TSF DLL 的 IPC 协议类型
（`ClientMessage` / `ServerMessage` / `Frame` / `PreeditSegment`，全 serde，两端共用，见 `docs/design/architecture.md`「Windows：TSF」；
协议版本见 `PROTOCOL_VERSION`，`PreeditKind::AuxCode` 对应 Core 的 `MarkedKind::AuxCode`，`Frame.aux_code_show` 随帧下发显示码开关，
`Frame.translation_pos` 随帧下发「译词是否带词性标注」（`[general] translation_pos`，缺省开；与 `aux_code_show` 同款普通 `#[serde(default)]`——帧只由 Server 自己的候选窗消费、DLL 不读该字段，加这种纯字段不 bump 协议版本，枚举变体才必须 bump）；
`Frame.columns` / `column_ems` 随帧下发横排矩阵视口，带 `serde(default)` 新老互通，DLL 不画候选读不到也照旧）。

## crates/qingjian-render

画位图的部分已拆到自绘引擎 `crates/bamboo-slip`：下文的 `scene/`、`animation/`、`text/`、`fonts/`、`canvas.rs`、`shadow.rs`
在 `crates/bamboo-slip/src/`，原 `renderer/{animate,retained,partial}.rs` 成了引擎的 `stage/`（`Stage::present` / `tick_at`），`theme/svg_image.rs` 成了 `SvgImage`。
这里留下帧数据、主题格式、`renderer/build/` 建树、点击区域、状态条与云朵 / 齿轮图标（`icon/`，实现引擎的 `Icon` trait）。改了引擎回来跑本 crate 的快照测试。
候选文字的显示截断在 `renderer/build/clip.rs`：建树前把带换行 / 制表符的画成一行（`↵` / `⇥`），超过 `MAX_TEXT_EMS` = 30 字宽的截尾加「…」（字宽估法与矩阵相同）；只改画出来的，壳按序号上屏不受影响。

横排矩阵：视口与高亮移动在 Core `candidate::layout::Grid`（`GRID_ROWS` = 6，列宽 `Grid::column_ems` 按整份候选估、单格封顶 `MAX_CELL_EMS` = 4 字宽）。横排且高亮移动键选了 ← / → 时（`two_area_arrows`），方向键怎么走在 Core `candidate::arrows::route`：拼音行与候选两区，↑ / ↓ 换区与换行，← / → 只在当前区里移动；两个壳只记焦点（mac `Session::focus`、Windows `Router::focus`，缓冲清空才回到候选）并执行它给的动作，焦点在拼音时帧里不给高亮（`usize::MAX`），拼音区里再按 ↑ 才开符号面板。其余组合（竖排、或横排用 ↑ / ↓ 移高亮）仍是原来那套：拼音行 + 候选连成一条序列。
青简渲染器在 `renderer/build/matrix.rs` 画矩阵：`Frame.columns` 不为 0 时，横排模板里候选的 `repeat` 改包进一个网格（主题不另写模板），
列宽至少 `column_ems`、超宽截尾加「…」（字宽估法与 Core 相同），格子里不画逐项译文；主题写了 `highlighted.annotation` 的照常画在网格下面，没有单独的信息行。
Windows Server 自绘候选窗（GDI）按 `Frame.columns` / `column_ems` 画矩阵：列宽滚动时不变、超宽候选截尾加「…」、网格下一行信息给出完整文本与译文；
这套按键 mac 壳（`Host::grid_keys`）与 Windows Server（`RouterConfig::grid_keys`）都由 `[general] horizontal_grid`（缺省关）加横排两个条件一起开。
`[general] grid_page_keys`（缺省关，#805）另让翻页键在单行时也展开（mac `turn_page`、Windows `Router::page`，与 `↓` 同一个 `move_rows`）；矩阵一旦展开，方向键按矩阵那套走（mac `Host::grid_keys`、Windows `Router::grid_keys` 都把「已展开」算进去）。

自绘渲染器：候选窗一帧 + 主题 → 预乘 RGBA 位图。主题是 `themes/<id>/theme.json`（内置青简绿 / 系统蓝 / 微信绿 / 玻璃 / 樱花 / 大肥鱼 / 初音未来 / 波奇酱编进 crate；新增内置主题前先查主题仓库 qingjian-themes 有没有同名 id，加了之后同步它 `tools/check.py` 的 `RESERVED_IDS`，否则那个用户主题会被拒绝加载；图片也可以 `include_bytes!` 编进去（`BUILTINS` 的文件表，第一次画到那个主题才解码：八个全解出来常驻 6 MB），`Theme::builtin(id)` / `builtins()` 取；没写 `extends` 的主题都以青简绿为底（`theme/mod.rs` 的 `ROOT_THEME`），用户主题还能取到所继承内置主题的图片；用户主题在 `<数据目录>/themes/<id>/theme.json`，`ThemeLibrary` 合并并按目录戳热加载；`theme/file/` 是 serde 模型，`theme/jsonc.rs` 解析前去掉注释与尾逗号，格式见 `docs/design/theme.md`「格式（schema 1）」），`renderer/build/` 按模板实例化场景树（`scene/`：Taffy 布局树 + 每节点一个 `Visual`，竖排三列用 grid、其余 flex，高亮条与光标绝对定位；布局不取整，文字测量按节点缓存；表格一行里的格子按第一段文字的基线对齐，`scene/baseline.rs` 补上边距再排一遍），再按树序画（节点的投影 / 内阴影在 `scene/draw/effect.rs`：把节点自己的画面画进离屏图取 alpha，三遍盒式模糊；窗口阴影就是根节点的投影；位图按 `scene/extent.rs` 并出的画出范围开（投影、描边、负 inset 伸出窗口的装饰都在里面），壳按 `Rendered::content_*` 对齐，mac 面板关系统阴影；贴着输入行摆时按 `Rendered::visible_overhang` 避让投影：只算不透明度过约 5% 的那几行，模糊外圈看不出却占着位图边（青简绿 2 倍时留 60 像素、看得见的下沿 27），按留边算会隔出一行空白（#757）；`Rendered::hits` 是候选与各条译词的点击区域（`renderer/hit/`，译词按义项分隔数分条，横排底下那行算高亮候选的），壳只在这些区域收鼠标、其余穿透）；过渡在 `animation/`（缓动、插值）与 `renderer/{animate,retained}.rs`：`Renderer` 留住上一帧，按节点 `id` 配对算过渡，`Rendered::next_frame` 告诉壳多久后调 `tick`（mac `candidates/animation.rs` 的 NSTimer、Windows UI 线程消息循环截 `WM_TIMER`），窗口收起调 `forget`；动画帧只重画、不重建树，效果遮罩与图片框按键缓存（`scene/cache_key.rs`）；循环动画 `animation/keyframes.rs` 求姿态，带姿态节点整棵子树画进缓存小图再变换贴上，循环帧走局部重画（`renderer/partial.rs` + `scene/split.rs` 分段模式）；tiny-skia 栅格 + cosmic-text 文字（fontdb 按平台清单只加载几个字体文件、不扫系统；
脚本字体文件要先过轮廓检查，任一面带 `glyf` / `CFF ` / `CFF2` 才加载（Android 的 Noto CJK 是 CFF2 可变字体；Android 上 cosmic-text 没有按 locale 的回退表，`fonts/android.rs` 加载后只留 locale 那张 CJK 面）——macOS 26+ 的苹方换成私有 `hvgl` 可变轮廓，swash 读不出字形，
这种文件跳过，简体回退到 Songti.ttc）；Windows 在常用中日文字体之后补 `simsunb.ttf` 与 `SimsunExtG.ttf`，
供扩展区汉字回退（系统没有对应文件时跳过），避免深翻页的生僻字候选因缺字显示成方框或空白，
自己解析 `trak` 字距表、按主题 gamma 加深笔画；主题样式的字族在 `theme/font/families.rs` 去重成字族表（样式里存序号），`TextPainter::use_families` 换主题时给每条链挑装了的字族，主题要的系统字族由壳查文件经 `Renderer::load_theme_fonts` 加载（mac 同 `bitmap/font_files.rs`、Windows 同 `system_fonts::family_files`），随主题带的字体在 `theme/assets.rs` 核路径；主题图片 `.svg` 结尾的走 `theme/svg_image.rs`（resvg 0.48，与渲染器同一个 tiny-skia，关掉 text / raster-images 与外部文件读取），画时按像素尺寸栅格并缓存；cosmic-text 打了 `opsz` 光学字号补丁（qingjian-team/cosmic-text 分支 `qingjian-opsz`，workspace `[patch.crates-io]` 钉 rev）。
`examples/preview.rs` 出 PNG 与真机截图并排比、`--measure` 与 AppKit 对宽度、`--theme <目录>` 画用户主题（系统字族在 `--font-dir` 里按名字找）。
快照测试 `tests/snapshots.rs`：`tests/scenes/` 的样例帧（与 preview 共用）逐像素比 `tests/snapshots/<os>/` 的基准，字体环境（`fingerprint.txt`）对不上或没有基准时跳过；
改了渲染就 `QINGJIAN_UPDATE_SNAPSHOTS=1 cargo test -p qingjian-render --test snapshots` 重出基准，变了的 PNG 看过再提交，失败时差异图在 `target/tmp/render-snapshots/`。mac 壳 `candidates/bitmap/` 贴位图，`[general] renderer = "system"` 切回 AppKit 绘制
（过渡期退路，偏好设置「候选窗口」页可选）；`[general] font` 是候选窗字族名（空为系统字体，`bitmap/font_files.rs` 用 CoreText 按字族名找文件只加载那几个，没装就回系统字体；
设置页 `preferences/font_picker/` 是搜索框 + 列表）。设计与验收见 `docs/design/rendering.md`。

主题预览图（`preview/`）：`preview::write_previews` 用 `sample.rs` 的 nihao 两帧把主题目录画成 2 倍的 `vertical.png` / `horizontal.png`（没锁定外观再加 `vertical-dark.png`），单张失败只记在它的 `Shot` 里；`preview::run` 是两个壳 `--theme-preview <主题目录>` 的入口（mac `main.rs`、Windows Server `main.rs`，Windows release 是 GUI 子系统，直接从 cmd 跑时先 `AttachConsole` 挂父进程控制台），写进 `<主题目录>/preview/`。主题仓库的 `tools/preview.py` 找本机装的青简调它，旧版不认参数会照常起服务，脚本 30 秒超时按版本太旧提示。

## crates/bamboo-slip-i18n

界面文案：每个有界面文字的 crate（设置程序、core、platform）各有 `locales/<语言>.toml`，在自己的 `i18n.rs` 里用 `catalog!` 声明、编译时嵌入，
代码里写 `tr("节.键")` / `tr_args("节.键", &[("名", &值)])`。简体中文是源语言，所有键都要有；英文是回退链的一环，也要全；其余语言缺的键退到英文再退到中文。
每个 crate 的 `i18n.rs` 有一条测试核对各语言没有废键、占位参数与源语言一致、英文不缺键。

界面语言是进程级的：`Config::load` 读到 `[general] ui_language` 就 `set_locale`（`GeneralConfig::ui_locale`）。没写时是简体中文：不少中文用户的系统首选语言是英文，跟随系统会让他们一升级就看到英文；写 `system` 才跟随系统（`sys-locale`，中文按简繁归成 `zh-Hans` / `zh-Hant`）。
比对中文文案的测试先 `pin_locale("zh-Hans")`：测试并行跑，固定之后读配置处的 `set_locale` 不再改它。
大数的写法也跟着语言走（设置程序 `stats/numbers.rs`）：表里 `numbers.myriad = "true"` 的语言按万 / 亿，其余按 K / M。
ⓘ 文档链接与支持表里的行名写文案键（`docs.rs` 的 `LINKS`），按当前语言的文字对上界面上的行。

## crates/qingjian-update

检查更新（设计见 `docs/design/update.md`）：`index/` 是索引的类型、下载（`fetch.rs`，复用 workspace 的 reqwest + 单线程 tokio，20 秒超时、2 MB 上限）与验签
（`signature.rs`，`PUBLIC_KEYS` 列表，`verify_strict`）；`checker/` 是调度（`Checker::poll` 由壳的每秒定时器调，到点起一次性线程）、落盘状态 `UpdateState`（`update.json`，先写临时文件再改名）
与查到的结果 `Available`。`Version` 自己实现语义化版本比较，不引 semver。`[update]` 配置与 `UpdateChannel` 在 `qingjian-platform`。
`examples/check.rs` 手动走一遍；`tools/release-sign` 是发版侧的 keygen / sign / verify。

## apps/cli

测试工具，`cargo run -p qingjian-cli -- kaifa`。

- `--predict` 强制开云联想并等结果打印，交互模式下上屏后也联想。
- `--code <码表或方案目录>`（旧写法 `--wubi`）开形码：给 TSV（`词\t编码\t词频`）按 86 五笔记日志、按键规则全关；给方案目录按 `scheme.toml` 来。
  按键当编码按前缀查表，候选不带音节，上屏吃掉整段编码。
- `--typing` 逐键计时（性能测试用 release 构建跑，目标每键 10 ms 以内）。
- `--chinese-first` 打开中文优先（`[general] chinese_first = true` 的排法），配合 `--replay` 比两种英文词位置。
- `--english-max-in-chinese <N>` / `--english-min-position <K>` 对应 `[general]` 同名配置：中文模式英文候选最多几条、有中文候选时最高排到第几位，验证 #768 用。
- `--ngram <zh.ngram>` 接知意输入法的词级三元模型（`qingjian_lm::TrigramModel`，见 crates/qingjian-lm 一节），选词重排里整段候选的语言模型分改由它给，整句词图里按 `TRIGRAM_WEIGHT` 与二元模型插值。
- `--zhiyi-onnx <模型目录> --onnx-runtime <onnxruntime.dll> [--zhiyi-tier saver|balanced|accurate]` 接知意 ONNX 版（与 Windows Server 同一条路：选词重排 + 整句字分布）；
  整句评测、回放与查询都会等后台的分回来再取结果（`rescoring::settled`，最多三轮：字分布回来后整句变了，选词还要再打一次）。
- `--replay <input-log.jsonl>` 回放评测：把日志里每次上屏的键重新喂给引擎，按来源算首选 / 前五命中率、平均名次、不在候选的条数，打印没命中的例子（`--misses N`）；
  只在内存里学习不写文件，加 `--user-dict` 可带上现有学习数据。
  **按日志记的方案逐条切换**（`replay/scheme.rs` 的 `SchemeSwitcher`，读每条的 `scheme` 字段）：整份日志通常只有一套方案，
  所以只在方案串变化时才换码表（`Engine::set_code_scheme` 收所有权，换一次要克隆 8.9 万条）。日志里是形码（不管哪个方案键，都用 `--code` 给的那一套）但没给 `--code` 时，
  那部分只计数——拿拼音的读法喂形码的键会算出看着像真的、实则无意义的命中率。
  含触发键的日志行按壳那样逐键重喂（进辅码态、码段进码段），
  并多打一行「辅码选词 N 条，其中同拼音纯输入首选命中 M 条」（学习闭环的尺子）。
- `--aux-table <路径>`（可多次）装辅码码表（`.qj` 或 `词<Tab>码` TSV）；交互模式与查询模式都按壳的方式逐键喂入，
  配的触发键进辅码态、之后的字母进码段，候选行会带上命中的码。
- `--tune 名=值`（逗号分隔）覆盖个人 n-gram 插值与敲错代价的常数扫网格（名字见 `apps/cli/src/tuning.rs`，Core 侧是 `Engine::set_interpolation` / `set_typo_costs`，壳只用缺省值）；
  `decay-days=<天>` 调用户词频 / 英文词 / 敲错表时间衰减的半衰期，0 为不衰减。
- `--mine-log <input-log.jsonl>` 从输入日志挖词库缺词：上屏文字按语言模型切词，连续单字段（切词失灵的地方）与「模型认识、词库没有」的
  多字词按出现次数列成 `词\t拼音\t次数` 清单（`--mine-min-count` 缺省 2、`--mine-max-chars` 缺省 6），读音从日志的切分按字符位置对回来；
  纯离线只读日志。
- `--mine-apply`（配 `--mine-log`）把清单里过阈值的**全拼**行（读音每段都是完整音节，判据是 Core 的 `parser::segment`）直接
  `Learner::learn_word` 并进用户词，**简拼**行单独列出等人工补全拼；是否落盘沿用 `--user-dict` 语义——给了就写回同目录
  `user-words.tsv`（保留已有的），不给只打印预演。不带 `--mine-apply` 单独跑一次本身已是清单预览。
- `--check-user-words`（配 `--user-dict`）按读音核对用户词，抓历史遗留的错读音（云端学进来的 `我的 wo di` 这类）：
  **读音不符**——词在词库里、存的读音不是词库给这个词的读音（正是词图「同文本不同读音顶掉整句候选」那一类）；
  **读音可疑**——词库不认识这个词、但某个字按存的读音在词库里不是这个字的读音（与引擎学习闸门 `accepts_reading` 同口径，生僻字放行）。
  主词库读音无调，逐字核对判不出「字有多音」（`的` 在 `di` 下本就合法），`我的哦 wo di e` 这种查不出来。加
  `--purge-user-words` 时用 `Learner::forget` 删掉并落盘（同时清其选择次数、各输入串选择、个人 n-gram）；不带则只列。
  `Learner::flush` 的契约是持久化失败只记日志、不返回错误，所以删完还要读回同目录的 `user-words.tsv` 核实，
  要删的词仍在文件里就报错、以非零状态退出，不凭 `flush` 没吭声便报「已落盘」。
- `--eval-text <文本>...` 装了码表（`--aux-table`）时多打一行码表覆盖率：词频前 10,000 与全库两段命中比例（与导入统计同源）。
- `--eval-text <文本>...` 整句评测：把用户自己写的中文文本按标点切句、按词库读音转成全拼，冷启动喂给引擎看整句能不能还原原句
  （首选命中率 / 字准确率 / 查询耗时；不依赖日志里当时选了什么，给整句排序与语言模型的改动当尺子），`--eval-save` 冻结成 `句子\t拼音\t上文` 三列文件，
  之后直接 `--eval-text` 它保证比的是同一份句子（本机的在 `data/eval/sentences.tsv`）。排序、整句、纠错的改动先跑它们再合。

## apps/macos

IMK 输入法，源码按 `app / host / imk / candidates / menubar / preferences` 分目录。

**在 Linux 上也能对它做类型检查**（不链接、不产出可执行文件，但编译错误与 clippy 都查得到）。
`qingjian-macos` 在非 Apple 平台上是硬 `compile_error!`、提交钩子也排除它，于是在 Linux 上改它等于盲改。办法是对 darwin 目标 check，
唯一的拦路虎是 `onig_sys`（candle → tokenizers 带进来的 C 库）要编 C，给它一个假的 pkg-config 条目让它改走「系统库」：

```bash
rustup target add aarch64-apple-darwin
# 在仓库根目录跑。与 .githooks/pre-commit 用同一个目录，钩子跑过一次就已经生成好了
onig="$PWD/target/darwin-check/onig"
mkdir -p "$onig/include" && : > "$onig/include/oniguruma.h"
printf 'prefix=%s\nincludedir=${prefix}/include\nName: oniguruma\nDescription: only for cargo check, never linked\nVersion: 6.9.9\nLibs: -lonig\nCflags: -I${includedir}\n' \
  "$onig" > "$onig/oniguruma.pc"
PKG_CONFIG_PATH="$onig" PKG_CONFIG_ALLOW_CROSS=1 RUSTONIG_SYSTEM_LIBONIG=1 \
  cargo clippy --target aarch64-apple-darwin -p qingjian-macos --all-targets -- -D warnings
```

第一次要对 darwin 把整棵依赖树检查一遍（只做 check、不生成代码，12 核机器上约半分钟），之后增量一两秒。
与 CI 的 macOS job 是同一道门（clippy `-D warnings` 含测试目标）；真要跑测试、打包、装机仍得在 Mac 上。

- 输入法菜单（状态项 + 系统输入源菜单）与偏好设置窗口都是配置文件的前端：只写 `config.toml`，`Host::apply_config` 一条通路热加载，激活期间每秒看一次文件 mtime。
  翻页键（`[general] page_keys`）走 `Session::page_target` / `can_turn_page`：选 `-=` 时 `-` / `=` **没有上一页 / 下一页就落回原语义**（英文直输段），别的翻页键翻不动照旧吃键（`imk/controller/display.rs::page_key`）。Windows / Linux 还是老行为，尚未同步。
  输入方案（`[general] scheme` 与 `code`）也在这里装配，交给 `Host::scheme_slot`（`qingjian_platform::SchemeSlot`，随包方案目录是 `paths::bundled_schemes_dir()`）；
  找不到或码表坏了只警告并按拼音跑。顶字与满码自动上屏接在 `imk/controller/text.rs` 收字母那里。
- `apps/macos/scripts/bundle.sh --install` 打包安装到 `~/Library/Input Methods/`（开发用），`--pkg` 做分发用的 pkg（装 `/Library/Input Methods/`，postinstall 跑 `qingjian-macos --register`
  注册、启用并切成当前输入源；签名 / 公证靠 `QINGJIAN_SIGN_IDENTITY` / `QINGJIAN_INSTALLER_IDENTITY` / `QINGJIAN_NOTARY_PROFILE`，没设就 ad-hoc；`QINGJIAN_TARGET` 指定架构，
  成品 `target/pkg/qingjian-<版本>-macos-<arm64|x86_64>.pkg`）；`scripts/uninstall.sh` 卸载。
- 装完菜单里没有青简（2026-09-22，macOS 27，`--install` 与 pkg 都遇到）：`--register` 和系统设置里的添加都只让输入模式 `app.qingjian.inputmethod.Hans`
  显示为已启用，父项 `app.qingjian.inputmethod` 没启用、`com.apple.inputsources` 里也没写进去，菜单里就没有；注销重登后两项都在、菜单正常。
  所以 postinstall、安装器结尾页与用户文档都把「注销重登」放第一步。当时 `target/Qingjian.app` 也在 LaunchServices 里登记着（同一个 bundle id 两份），
  bundle.sh 因此在 `--install` / `--pkg` 用完就把 `target/` 与 pkg 暂存目录里的副本删掉、再按路径从 LaunchServices 注销（`forget_bundle`；
  顺序不能反：登记是异步的，先注销再删会留下指向已删路径的记录）。只是这一步当时没让菜单出现，真正起作用的是重登。
- 辅码：`host/aux_code.rs` 按 `AuxCodeSources` 装码表（随包 `Resources/codes/`，bundle.sh 从 `data/generated/codes/` 拷，连同 CNS 署名），`reload_config_if_changed` 顺带比文件快照；
  按键分流在 `imk/controller/text.rs`，规则与 Windows Server 一致（触发键、码段、筛空吞键、辅码态标点先上屏），Esc 在辅码态只清码段；`Row.code` 进渲染器，旧原生候选窗不画码。
- 日志在 `~/Library/Logs/Qingjian/`（按天分文件留 7 天，删了会重建），用户数据与配置在 `~/Library/Application Support/Qingjian/`。
- 配置项：云联想 `[predict]`（偏好设置「云服务」页有「测试连接」按钮：`qingjian_predict::ConnectionTest` 起线程发一条最小请求，`Host` 用独立定时器 `CloudTestMonitor` 轮询结果显示到窗口底部；
  `reasoning_effort` 缺省 `none`，DeepSeek V4 默认思考，不关正文为空）；模糊音 `[fuzzy]` 默认都关；`[general]` 学习语言（`off` 不显示译文）/ 每页候选数 / 翻页键 / 外观 / 竖排横排 / 拼音显示位置 /
  英文模式候选开关 / 中文优先 `chinese_first` / 中文模式英文词 `english_in_chinese`（`Engine::set_english_in_chinese`）/ emoji 候选 `emoji`（`Engine::set_emoji_candidates`）/ 中西文自动空格 `auto_space`（直通字符经 `text.rs` 的 `pass_through`，开着时首键也读光标前文）/ 双拼方案 `shuangpin`（小鹤 / 自然码 / 微软 / 搜狗 / 智能ABC / 小浪 / 首道 / 拼音加加，空为全拼）/ 日志级别 `log_level`（缺省 info 不含敲的内容，debug 逐键记，热切换）/ 输入日志 `input_log`；
  `[shortcut]` 中英切换键 `switch_mode`（macOS 只认 `shift`：缺省 `[]` 用 Caps Lock 切；勾上后单击 Shift 切中英、Caps Lock 只管大小写，`control` / `ctrl+alt+space` 忽略；
  切换时右上角弹 `host/mode/osd.rs` 的提示，组句中切到英文还会把拼音原样上屏，见 `host/mode/switch.rs` 与 `imk/controller/click.rs::commit_raw_after_switch`；
  单击的边沿用修饰键事件（`IMKInputController` 的 `recognizedEvents:` 声明 `NSKeyDownMask | NSFlagsChangedMask`——缺省只回 `NSKeyDownMask`，
  实测就只有 keyDown 送进来、Shift 的按下 / 抬起完全收不到，那时只能靠 30 ms 轮询采样、整拍跨越的快点会漏掉）、模式键 v / u、`question_mark`（缺省关，开了空缓冲区敲 `?` 进问字）、上屏第一 / 第二个译词的修饰键 `translation` / `translation_second`、翻译选中文字 `translate_selection`（Linux 缺省 Alt+Shift+T）；
  `[apps] english_candidates_on` 按应用标识列出英文模式给候选的应用（白名单，缺省浏览器 / 聊天 / 办公 / 笔记，`*` 前缀匹配，`["*"]` 处处都给），`english_candidates_off` 从中排除；
  `[dictionaries] domains` 打开随包的领域词库（`Resources/dicts/` 11 本，缺省只开 `idioms`），`disabled` 关掉用户目录 `dicts/` 里的某本导入词库；
  偏好设置「词库」页随包的可开关、导入的可开关 / 移除，可导入 TSV / Rime yaml / .qj。
- 系统文本替换（系统设置「键盘 → 文本替换」）：`host/config/text_replacements.rs` 从 `NSUserDefaults` 全局域读 `NSUserDictionaryReplacementItems`
  （每条 `{ on, replace, with }`），激活输入法时重读，变了就经 Core `merge_replacements` 并进配置里的自定义短语再 `set_custom_phrases`；
  `[general] system_text_replacements` 开关（缺省开，「自定义短语」页勾选框），内容可能含证件号、地址，日志只记条数。
- 输入法进程由 launchd 拉起，看不到 shell 的环境变量：密钥写进配置同目录的 `.env`（`QINGJIAN_API_KEY=...`，输入法启动时 dotenvy 读入）或 `config.toml` 的 `api_key`。
- 本地整句模型：安装包不带模型，通变与知意都由设置程序下载到用户目录 `models/hanzhang-tongbian/`、`models/zhiyi/`；选的没下载就不用模型（`paths::p2c_model_path` 仍认包里的同名目录与旧 `model-p2c/`，给开发时自己放的那份）。`host/model/mod.rs` 在后台线程加载并预热（首次 Metal 编译）后
  `set_async_sentence_scorer` 接上，`refresh` 每键先读应用光标前 64 字给 Engine 当前文、查询后 `schedule_rescoring`，`RescoreMonitor` 停键 80 ms 请求、20 ms 轮询，
  结果到了重查一次只重画当前页（翻过页 / 动过高亮不动）；「云服务」页有开关（`[model] enabled`）。
- 端到端验证可用 `osascript` 的 System Events 往 TextEdit 发按键再读回文本（终端需要辅助功能权限；输入法得在中文模式）。

## apps/windows

候选窗 `ui::candidates::CandidateWindow::show` 每次更新内容后用 `SetWindowPos(HWND_TOPMOST)` 重新升到置顶序列首位，并保留 `SWP_NOACTIVATE`；Windows 搜索页后弹出的置顶窗口可能盖住较早创建的候选窗。

用量接线：启动与配置热加载为 CloudPredictor 和 CloudGlossFiller 传同一个用户目录 `api-usage.json`；设置页的 ConnectionTest 使用该文件。统计页手动刷新读取累计记录，收到 JSON 响应与正文有效性分开统计；原译词自动补充、用户配置及学习文件沿用。

云联想接线：Ctrl+Alt+J 由可配置 TSF 保留键转发到 Engine 显式入口；自动重组仍走原自动入口。Composed 记录待收手动序号并在输入重建／取消／收结果时清除；自动结果用末尾布局，手动结果用前置布局，导航后保持候选不变，本地重排沿用当前布局模式。应用前文继续用于两类请求，译词自动补充继续装配。设置沿用默认 300/3000 ms，不写入个人偏好。

云联想与整句译文分别跟踪序号；整句译文保持原接线。导航后只冻结候选布局，小字整句补全仍更新并可 Tab 接受。手动请求和 TSF 保留键均受云联想总开关约束，单独开启整句译文不会启用联想快捷键。

新版 `apps/settings` 的云联想节奏与录键入口登记为 Windows 支持项；旧版 WinUI 设置保留同样的配置写入。两版共用 `[predict] automatic/debounce_ms/min_interval_ms` 和 `[shortcut] predict`，不改其他平台的自动联想入口。

`[general] numpad_digit`（`NumpadDigit`）控制普通中文组句中的小键盘数字：`select`（缺省）选当前页候选，
`direct` 将数字送入 Core 缓冲区，按输入顺序原样上屏。`RouterConfig::from` 同时供启动与配置热加载使用；
Windows 设置「通用」页用同一枚举生成选项。主键盘数字、英文、注音、表达式与问字模式保持现有行为。

一个产品两个 package：`server`（Server 进程：IPC 分派 + Engine + 命名管道 + 自绘候选窗与悬浮状态条）与 `tsf`（TSF 文本服务 DLL，lib 名固定 `qingjian_tsf`），
外加 `settings`（旧版 WinUI 3 设置程序 `qingjian-settings-legacy.exe`，兜底用）与 `installer`（Inno Setup，同时装 `apps/settings` 的 `qingjian-settings.exe`）。
Server 侧辅码接线：`RouterConfig.aux_code_key` / `aux_code_show`（`apply_config` 热加载）、`assembly` 按 `AuxCodeSources` 装 `codes/` 与辅码方案
（`[aux_code] disabled` 过滤，热加载轮询 `DataDirs::code_snapshot`）、按键在 `dispatch/key/input.rs` 分流（触发键进辅码态、辅码态里字母进码段、
大写（`shift_letter = "compose"`）先清码段回拼音态再进缓冲区、标点先上屏高亮候选再转全角）、候选窗 `ui/candidates/row.rs` 的 `Row.code` 把码用方括号括起来紧跟在候选词后面（不进 annotation），拼音行 `view.rs` 给码段加下划线。
热加载的 `dicts/` 与 `codes/` 目录与启动同款（修过一处传基础目录的错）。不合成一个 crate，因为 DLL 不能带 Engine 的依赖树，见 `apps/windows/README.md`；
协议类型在 `qingjian-platform::protocol`，设计见 `docs/design/architecture.md`「Windows：TSF」。

Server 的 `config_load` 严格读取配置，缺文件与解析失败分别分类；启动日志记录配置路径、程序路径、来源及模板创建状态，不打印配置正文。配置监听首次轮询重新读取，避免启动读取与监听建立之间的恢复被遗漏；后续读取失败保持上一份有效配置。

Windows 的 `dispatch/mode` 管理可选的初始英文与按应用模式记忆（`[general] default_english` /
`remember_mode_per_app`，缺省关闭）：exe 名不区分大小写，相同 exe 的会话共享；未知宿主按会话隔离。
应用记忆只驻留 Server 内存，配置中的模式策略变更时重置；模式焦点与组句焦点分开，避免切应用时丢掉待提交的输入。
`InputSettings.restore_mode` 让启用这些设置的 DLL 重连时先取 Server 状态，不用旧状态覆盖默认值；旧协议缺字段按 false 处理。
输入方案由 `[general] scheme` 与 `code` 决定，Server 启动与热加载各装配一次（`dispatch/schemes.rs`）；随包方案目录 `assets/schemes/` 启动时由 `find_bundled_schemes` 找好
（走 `assets/` 与 emoji / levels 一致，开发布局也对得上），装配交给 `Router::scheme_slot`，用户在设置里换方案、新放进方案热加载即生效，码表没变不重读。
选了形码却找不到方案或码表读不了时只警告并按拼音跑——配置说五笔、引擎还在拼音是静默错位，宁可吵。
顶字与满码自动上屏在 `key/input.rs` 的 `push_letter`。
中英模式的两项设置（`[shortcut] switch_mode` 切换键：勾选 shift / control / ctrl+alt+space，`[general] english_mode` 内置英文模式开关）
由 Server 经协议下发给 DLL（`InputSettings`，见下文「按键行为设置」），改完在下一拍（约 320 ms）生效；
`ctrl+alt+space` 走 TSF 保留键登记（`com/key/preserved.rs` 的 `GUID_SWITCH_MODE`）。系统的 Ctrl + Space（「输入法/非输入法切换」）由设置页单独列项，`settings/src/panel/system_hotkey.rs` 通过 `ImmGetHotKey` / `ImmSetHotKey` 读写 `IME_CHOTKEY_IME_NONIME_TOGGLE`，不写入 `switch_mode`、不重复注册 TSF 保留键；同一项也会影响其他简体中文输入法。TSF 适配它：
它翻的「输入法开 / 关」compartment（`com/mode/sink.rs`）关 = 英文、开 = 中文，我们切模式时把开关写成一致；开关一变也作废被截走 Space 的那次「单击 Ctrl」。
模式缺省全局共享，启用按应用记忆后由 Server 按宿主保存；当前激活值在 `Router.english`。DLL 激活 / 得到焦点 / 轮询时 `SyncMode` 取回，用户切了 `ModeChanged` 报上去。会话号用线程 id（`com::session_id`）——TSF 的 client id
各进程都是同样那几个值，拿它当会话号会在 Server 那边撞号。四条切换入口都汇到
`service/mode.rs::set_english_mode` 一处拦住；状态条点击在 Server 侧（`dispatch/status/mod.rs`）按同一项拦，
设置界面在 `settings/src/panel/pages/general.rs`。

TSF 原有数字 / OEM 标点 / 空格键码按当前布局用 `ToUnicodeEx` 解析（bit 2 避免改变键盘状态），
仅接受单个非代理项 UTF-16 单元。字母、小键盘和 AltGr 处理不变，不保证组合音符输入。
没在组句的数字在 `OnTestKeyDown` 先问 Server（`key_sink::preflight_digit`）：Passthrough 则 Test 不吃、键原生进应用
（避免 `InsertTextAtSelection` 在未设 `KEYBOARD_DISABLED` 的 `IS_NUMERIC_PASSWORD` 框里倒序）；注音等 Consumed
时把结果缓进 `pending_digit`，`OnKeyDown` 再应用。拼音显示位置（`[general] preedit`）在 Windows 上分两处落地：
Server 把它读进 `RouterConfig.preedit` 并随 `Frame.preedit_mode` 下发给 DLL，DLL（`com/service/key_sink.rs`）按
`inline()` 决定要不要放行内拼音，Server（`ui/candidates/render_data.rs::window_preedit`）按 `in_window()` 决定候选窗口顶部画不画拼音行；`window` 模式没有组句范围，光标矩形改从 `com/edit/anchor.rs::caret_rect`（当前选区）量。
自己画候选的应用（全屏游戏，TSF 的 UI-less）：DLL 把按键、轮询、点击上屏回的每一帧交给 `com/candidate_list`，组句开始时经 `ITfUIElementMgr::BeginUIElement`
起一个 `ITfCandidateListUIElement`，帧变了才 `UpdateUIElement`，组句结束 / 断线 / 停用时 `EndUIElement`。应用回 `bShow = FALSE` 时
`composition::report_caret` 不报光标位置，Server 没有 `last_rect` 就不画候选窗（不能改发 `HideCandidates`：它会顺带取消云联想）。
线上帧只带当前页，给应用的列表就是这一页（只有一页、当前第 0 页）。端到端验证：注册 DLL、起 Server 后
`cargo run -p qingjian-windows-tsf --example host -- tsf+uiless "nihao "`，宿主像游戏一样取候选并打印，也打印青简候选窗有没有弹出。
连不上 Server 时 DLL 自己拉起它（`tsf/src/com/service/launch.rs`）：`ShellExecuteW` 起与 DLL 同目录的 `qingjian-server.exe`
（`uiAccess=true` 的 exe 用 `CreateProcess` 报 740），进程内 5 秒冷却 + 跨进程命名互斥体防止砸出一串 Server；
起完清掉重连退避，下一键就试。Server 只在登录时由「启动」文件夹拉起，中途挂了以前只能等下次登录。
Server 自己一开始就占命名互斥体 `Local\QingjianServer`（`ipc/instance.rs`），占不到直接退出：管道要等装配完（一秒多）才建，
以前登录时快捷方式与 DLL 各拉一个的话，第二个会把词库与本地模型整套加载一遍才因管道被占退出。

词库导入（设置「词库」页）走 `qingjian-dictionary::import` 转成 `.qj`（空词库拒绝），多选批量、成功的从 `[dictionaries] disabled` 摘掉、页面显示每个文件的结果；
`import::import_dictionary` 是各平台导入入口用的那层：源文件里有没写读音的词（只有词的 txt / 混了无读音行的 Rime 词库，
`parse_words` 判得出）就转 [`import_word_list`] 派生读音，否则仍走 `import`（`.qj`、全带读音的 TSV / Rime 行为不变）；
返回 `ImportedDictionary { imported, report: Option<Report> }`，设置页的结果行列出跳过的条数与前几个词。
另有 `qingjian-dictionary::import::import_word_list` 收「只有词、没有拼音」的词表（每行 `词` / `词<Tab>词频` / `词<Tab>读音[<Tab>词频]`，`---`…`...` 的 Rime 头一并认），
读音列可以带数字声调（`dong1 xi5`），数字只能跟在字母后面。读音按参考词库（基础词库 + `--extra-dict` + `assets/lexicon/dicts/*.tsv` 领域词库）取，顺序是：词表写明的读音 → 整词词条 → 最长匹配切分
（`中华人民共和国` → `中华` + `人民` + `共和国`）→ 单字取词频最高的读音（`和` 取 `he`）。**一个词只写一条读音组合**，不做多音字的笛卡尔积；
字不在参考词库里的词跳过并进 `Report::unknown`（不静默丢词）。多音字词（`美露莘 mei lu xin`）在词表第二列直接写读音。
设置程序、Windows 旧设置、macOS 旧偏好与 Android 的参考词库都是「随包基础词库 + 随包领域词库」，不与用户开关联动。
预览与落盘在 CLI 是 `--import-words` / `--import-to`；
Server 每次轮询比对用户 `dicts` 的路径 / mtime / 长度快照，配置没变也重载新增、同名更新与移除；配置解析失败时词库沿用上次有效的开关（#36）。

中英切换键（`[shortcut] switch_mode`）与内置英文模式开关（`[general] english_mode`）得在**按键到达之前**就知道
（单击判定在 `OnTestKeyUp`、是否登记语言栏按钮），但 DLL 跑在每个应用进程里、拿不到 Server 那份配置，
`%APPDATA%\Qingjian` 对 AppContainer 里的商店应用也读不到（那是给输入日志和 `.env` 用的目录，不该加 ACE）。
所以由 **Server 读配置、经协议下发**（`InputSettings`：`OpenSession` 回包带一次，之后每拍 `SyncMode` 跟着走），
DLL 不读文件、不查 mtime。`SessionOpened` 只回过协议版本对得上的 DLL——老的 `open` 是只写不读，
多回一条会被它当成下一次 `Poll` 的应答而报错，那条连接就废了；老 DLL 从 `ModeSync` 那一拍也能拿到同一份（新字段直接忽略）。

TSF 显示属性枚举的 COM 输出槽可能尚未初始化，必须用 ptr::write 写入接口，不能通过 Rust 赋值释放旧槽。显示属性输出指针与 GUID 入参检查空值和对齐；原生 vtable 单测验证输出初始化及错误 HRESULT，不依赖真实宿主窗口。

Windows 可选的 `[shortcut] toggle_punctuation` 经 `InputSettings` 下发，由 TSF 保留键触发原有标点菜单命令；
修改或关闭时撤掉旧登记，句号使用 `VK_OEM_PERIOD`，不能使用字符的 ASCII 值（那是 Delete）。
首次获焦可能早于 `Activate` 设置 `thread_mgr`，因此保留键同步必须在设置值缓存命中前执行，管理器就绪后才能补登记。
`extra_page_keys` 与主翻页键并用，只影响候选翻页，不覆盖表达式、直输段或辅码触发键。

### Windows 简繁快捷键

`[shortcut] toggle_traditional` 是可关闭的组合键（空字符串关闭，缺省关；设置程序里开关打开写 `ctrl+shift+f`）。Server 经 `InputSettings` 下发有效组合，与选区翻译冲突时禁用简繁键。TSF 在激活 / 热加载时替换保留键登记、停用时撤销，命中后转发按键；Server 在简体与上次用的繁体之间切（`[general] last_traditional`，切到简体时记下切走前那一档，没记过是台湾），复用 Core 的繁体转换并重查候选，不上屏或清空当前组句，结果写回 `[general] traditional_variant`。老 Server 不下发此字段时 DLL 不登记。激活早期焦点回调可能在 `thread_mgr` 就绪前连接 Server，因此保留键同步先于配置去重，下一次相同设置也能补登记或重试。
Windows 选区兼容（协议 10）：TSF 为空时，新 DLL 发 `SelectionFallback`（触发进程、前台窗口及可选的 Scintilla 选区），Server 在独立 MTA 工作者中查询 UIA `TextPattern`。工作者核对进程、焦点、密码状态、单一选区与 500 UTF-16 单元限制；仅主动翻译触发，单工作者、5 秒收结果超时，取消后丢弃迟到结果。备用评审用不含 `commit` 的 `SelectionReview` 回包维持等待；只有用户确认才通过 arboard 复制译文，界面提示自行粘贴，复制失败保留结果重试。普通 TSF 来源在 DLL 的 `composition/selection.rs` 保存范围与原文，回填前重验文档、全局前台窗口、线程焦点、起止位置和原文。版本 9 已支持的码段 / 符号降级门槛固定为 9，不因新增协议 10 而错误降级。Linux 只同步公共握手版本，不实现 Windows 的备用入口。

Windows 配套版本在 TSF 翻译未完成时确认，会保留请求并以协议 10 的 `SelectionReview(active=true)` 维持 DLL 评审状态。这种状态确认的空帧不结束已有 UI-less 候选元素，避免等待或复制期间反复关闭和重建；退出评审时才按空帧结束。Server 的 `--verify-translation [资源目录]` 用固定公开样例验证实际配置、本地模型与 Router 的触发、等待、确认及取消路径，使用无窗口输出端；不读取宿主选区，不访问剪贴板，也不代表真实应用验收。

### Windows 配置诊断

Windows Server 的每条日志加当前 PID，启动与 `--check-translation` 另记配置的显示路径、`canonicalize` 解析后的实际路径、字节数及修改时间，不输出配置正文或摘要。Windows 打包应用内启动的诊断工具可能读到 AppData 私有副本，不能把相同显示路径当作相同文件；排查时比较实际路径。元信息是独立快照，并发替换时不承诺与解析内容来自同一文件句柄。

## assets


- `assets/sample/`：手写样例词库与释义表，不是产品数据。
- `assets/schemes/wubi86/`：随包形码方案（`scheme.toml` + `table.tsv`，码表由 `dict-convert wubi` 生成，来源与许可见同目录 README，Apache-2.0）。
  `bundle.sh` 把 `assets/schemes/*/` 的 `scheme.toml` 与 `table.tsv` 拷到 `Resources/schemes/`，Windows 安装器拷到 `{app}\assets\schemes\`
  （与两边的找法对齐：macOS `paths::bundled_schemes_dir`、Windows `dispatch::schemes::find_bundled_schemes`）。
- `assets/emoji/emoji-zh.tsv` / `emoji-en.tsv`：Unicode CLDR 中文 / 英文 annotations 转出的 emoji 表（Unicode License v3，可发布；中文词与英文词各配 emoji，两张表加载时合成一张），
  `cargo run --release -p qingjian-dict-convert -- --out-dir assets/emoji emoji --language zh data/cldr/annotations-zh.json data/cldr/annotationsDerived-zh.json`（en 同理）。
- `crates/qingjian-core/data/panel/{catalog,keywords}.tsv`：符号面板的分类与搜索关键词，`uv run tools/corpus/symbol_panel.py`（参数见脚本头）由维护者整理的符号库、
  Unicode emoji-test.txt（只收完全限定、不带肤色、Emoji 15.1 以内）与 CLDR 中文 annotations（名字与关键词）生成。
- `crates/qingjian-core/data/{emoji,kaomoji,symbols,words}.tsv`：编进 Core 的小表（口语 emoji 叫法、颜文字、符号，以及词库没有的那些词的读音），来源见同目录 README。
- 英文词表词频：`uv run tools/corpus/english_frequency.py data/generated/english.tsv -o data/generated/english-frequency.tsv`，再 `... english <词表> --frequency <那个文件>`。

## tools/gloss-gen

用 LLM 批量生成释义表：`cargo run --release -p qingjian-gloss-gen -- generate`（密钥读 `QINGJIAN_API_KEY`，结果 JSONL 在 `data/generated/`，不进 git、可续跑，`--limit 80` 试跑）
再 `... export`（写 `glossary-{en,ja,vi}.tsv`，产品数据在 `assets/glossary/`，见那里的 README；格式 `词\t词性. 译词[|假名]`）。只补越南语时 `generate --languages vi --out data/generated/gloss-vi-llm.jsonl`，再对这个 JSONL `export`。CLI 与 bundle.sh 用的是这些文件；打包时 `dict-convert pack glossary --language vi` 会写 `glossary-vi.qj`。

## tools/dict-convert

产品数据的生成工具，输出到 `data/generated/`（gitignore）。

- `lexicon`：从 `assets/lexicon/`（自建词库源：规范字 + 常用词 + THUOCL 领域词 + 生僻字）加 Unihan 读音（`data/unihan/Unihan_Readings.txt`）、LLM 多音字标注（`gloss-gen pinyin`，
  结果 `data/generated/pinyin-llm.jsonl`，不进 git）、语料词频（`lm-unigram.tsv`）建基础词库 `dict.tsv`（13 万条），并把 THUOCL 领域词按语料次数 < 50 拆成
  `dicts/<领域>.tsv` + `.qj`（11 本、13 万条，`--domain-keep-min`），流程见 `assets/lexicon/QINGJIAN.md`；Unihan 只给 `n` / `ng` 的字（嗯）按输入习惯收成 `en`；单字各读音按 kHanyuPinlu 频次分摊字频，频次表里没有的次要读音按主读音的 5% 猜（`MINOR_READING_SHARE`），再压到同音字里有频次撑着的最高那个的 1/4 以下（`MINOR_READING_CEILING`：高频字的 5% 绝对值很大，和 读 hu 原来 4.5 万次、排在 胡 湖 前，#759）；生僻字表 `01_characters/rare_chars.tsv`（`tools/corpus/rare_chars.py` 从「规范字表外扩充汉字」与台湾教育部罕用字 C 表合成，3.6 万字）里词库没有的字按词频 0 收、读音用表里的（词频 0：不进整句词图、`bigram` 分词跳过，语料里的日文新字体与繁体残留不会把它们统计成常用字；表里标 `everyday` 的 诶 囧 按规范字处理），音节表为此加了 `fiao`（覅）`biang`（𰻝），切分与双拼同键时让给常用音节；`--extra-words` 并入人工挑的领域词 `assets/lexicon/domain_words.tsv`（2026-09-27 起该文件另含按 jieba 词表（MIT）对照出的缺失高频词：读音逐字取 Unihan、多音字逐条人工判定（判定明细与理由在 `assets/lexicon/00_meta/polyphone-judgments.tsv`），词频按 log-log 回归从 jieba 次数折算；另含「姓氏+总」称谓层 98 条与符号词 6 条，定值依据见文件头注。这些词进语言模型同样走 `bigram --phrases`，改了词库发数据时要重跑 bigram，否则新词只拿兜底分、会被同音的 lm 词压过）。五份额外词以前要手动列，少给一个那批词就从 `dict.tsv` 里悄悄没了（漏过 `mixed_words.tsv`，C盘 / U盘 打不出来，#807），现在缺省全并、缺文件报错；`lexicon/tests.rs` 对着仓库里的 `dict.tsv` 与 `dicts/` 查这五个文件的词都在。
  各项输入缺省都并入（LLM 读音、语料词频、`assets/lexicon` 五份额外词、`data/lexicon/` 的青简词库），缺文件报错，不要的用 `--no-pinyin` / `--no-frequency` / `--no-extra-words` / `--no-general` 排除，免得漏给参数悄悄少并一份。
  青简词库（数字声调版，缺省 `data/lexicon/qingjian-full.tone-numbers.tsv`，交付目录软链过来）并入通用大词库（`lexicon/general.rs`）：基础词库已有同读音的补声调、词频不动；读音不同的新读音按换算词频的 1/8 收；其余按两边同读音词在对数坐标拟的直线（2026-10-21 交付版 7.2 万对，斜率 0.97、截距 1.39）换算词频，写 `dict-general.tsv`（输出目录顶层）与 `dicts/general.qj`；领域词库不动（挪进基础词库会让 智永 这类人名进第一档）。它们不进 `bigram` 分词词表（一元最大概率切分会把片段整块切走），所以 `dict.tsv` 除声调外与不并时逐行相同、语言模型不用重跑。领域词多音字按语料读音猜时打平取 Unihan 靠前的读音（以前按 HashMap 顺序，每次跑可能不同）。
- `english`：转 `assets/lexicon/05_english/00_all_words.tsv`；同编码优先保留含大写的专名写法（Windows ≠ windows），
  展示写法补充表 `07_display_forms.tsv` 后置读入；`cedict`：释义表备用来源。中英混杂词源在 `assets/lexicon/mixed_words.tsv`（`lexicon --extra-words`）。
- `tones`：`assets/lexicon/02_common` 通用词表的数字声调逐字收成单字声调表（`字\t带调音节…`，轻声记 5，按词表排序号首次出现的顺序，第一个当首读音），
  `--out-dir crates/qingjian-core/data` 直接写到 Core 的 `data/tones.tsv`（`include_str!`，进 git，约 5 千字 50 KB）；通用词表没收的字没有声调，注音排序当查不到。
- `chaizi`：開放詞典網拆字字典（`assets/chaizi/chaizi-jt.txt`，CC BY 3.0）的每种拆法按词库单字读音连成部件拼音串（林 → `mumu`；多音部件按词频取前 3 个读音交叉、一种拆法最多 16 串；有部件读不出的拆法跳过；原表同一个字分两行的合并），
  行按字频排，写 Core 的 `data/chaizi.tsv`（约 1.4 万字 280 KB）。Core 的 `chaizi` 模块整串对上在前、前缀在后，问字模式出候选（`engine/query/modes.rs`），全是字母又拆得出字时数字键选字、不当码点。
- `wubi`：Rime 形码码表（`.dict.yaml`，极点 86 五笔）→ `词\t编码\t词频`（`wubi.rs`，`--name` 决定文件名，缺省 `table.tsv`，即形码方案目录里码表的缺省名）。单字母码保留源表顺序，其余编码按回填词频排序。
  **码表自带的权重不用**——那是码表顺序不是语料词频，用了同一个词在形码下和在拼音下会排得不一样；词频从青简词库按**词面**交叉回填，
  词库里没有的词给 `UNKNOWN_FREQUENCY = 1`。解析复用 `qingjian_dictionary::import::rime`（与用户导入 Rime 词库同一个解析器，
  形码码表的第二列是编码，格式一样）。词库还没收的词不收（码表整张进，不做取码推导，见 `docs/plan/wubi.md`）。
- `bigram`：统计语料；`--phrases` 给短语层、`--brand` 给品牌词（`assets/lexicon/brand.tsv`，青简 210）与中英混杂词（`mixed_words.tsv`，C盘 / B站：合成计数要成分词在语料里，C 不是 token，只能直接给一元，次数对着同音竞争词定；词表里没有的品牌词直接报错，不再跳过——跳过的话模型里也跟着没有），领域词也走合成计数（语料里只有几十次的词当 token 统计会吸走成分词的二元证据）。
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

`qingjian-linux-server` 为独立产品，装配本地 Engine、词库、释义、频率学习、个人 n-gram、词汇记录与可选输入日志，
本地整句模型优先加载用户 `~/.local/share/qingjian/models/hanzhang-tongbian/` 或随包 `data/models/hanzhang-tongbian/`（Linux 继续随包，还没接下载）；含章·知微已下线，不再回退；旧用户目录 `model-p2c/` 兼容读取。按 `[model] enabled` 在后台加载、停键 80 ms 后重排、20 ms 轮询（Windows Server 的 `dispatch/rescore` 已改成 30 ms 防抖 + 后台叫醒，这里还没跟）；云联想按 `[predict]` 接入，用量累计到用户目录的 `api-usage.json`（设置统计页读同一份），释义兜底与生词本整句翻译随 `enabled` 一起接，结果由 `tick` 里的 `poll_glosses` 收。配置热加载在 `dispatch/reload`：`tick` 每秒看 `config.toml` mtime 与用户 `dicts/` 快照，`[predict]` / `[model]` / `[dictionaries]` / 学习语言按变化重建，其余无条件重设；`preedit`、`input_log` 仍要重启。`dispatch/session` 交换每个上下文的 EngineSession；真正的能力变化丢弃输入，普通焦点切换隔离保存。
默认面板插件仅转换事件（Shift / Ctrl 本身报成 VK `0x10` / `0x11`），中英切换、候选点击、分页和失焦提交都由 Server 决定：`[shortcut] switch_mode` 的单击 Shift / Ctrl 在按下时记进会话的 `switch_tap`、中间按了别的键或失焦就作废，松开时切换；`Ctrl + Alt + Space` 在按下时切换。`[general] default_english` 定新会话的初始模式，`english_mode` 关掉时切换键全不认、热加载时把所有会话拉回中文。
启动与热加载共用 `assembly::attach_cloud`（签名与 Windows 的 `dispatch::attach_cloud` 对齐：`[predict]` 配置 + 用量文件路径）；切换服务先取消旧请求与选区翻译，重建所有会话的本地候选布局以清除旧云词，保留本地高亮，聚焦会话再发新请求。无密钥时退回本地，不预留云词槽。按需联想在 `LinuxEvent::Key` 里拦 `[shortcut] predict` 组合键（按字符匹配，`.` 的键码与 Delete 冲突），结果按模型顺序置顶；自动请求由 `RouterConfig::cloud_enabled` 门控，`automatic = false` 时只剩按需触发。插件把缺失的可选 `kind` / `sentence` 按本地候选 / 无整句处理。
插件在清空预编辑之前上屏；上屏回调重入后核对生命周期和帧身份，跳过过期重画而不撤销或重复提交，见 [Linux 协议与状态](linux-fcitx5.md#协议与状态)。

Unix socket 用共享长度前缀与 Frame（当前公共版本 10，与 `PROTOCOL_VERSION` 同步，Fcitx5 插件里写死在 `qingjian.cpp` 的 OpenSession）；插件复用一条连接，每个上下文独立会话。Linux v3 扩展逐会话握手、确认 Sensitive/Password/Disable 后接受按下/释放、焦点和点击事实。
候选回报绑定连接代次、上下文和服务端帧序号，仅当前聚焦页的有效释义进入 `note_displayed`，不把生成帧算作已展示。
Linux 回包由 `dispatch/display/annotation` 给释义添加可选 `annotation`（词性与汉字段假名，`[general] translation_pos` 关掉时不带词性），Fcitx5 面板优先显示它；原始译文和快捷键提交不变，旧 Server 缺该字段时插件回退到 `text`。
`[general] preedit` 使用已有 `both` / `inline` / `window`；没有新增 Linux 自绘配置。详见 [linux-fcitx5.md](linux-fcitx5.md)。

线上类型（`LinuxEvent` / `Capabilities` / `DisplayIdentity` / `DisplayAcknowledged` / `LINUX_UI_PROTOCOL` / `socket_path`）
在 `qingjian_platform::protocol::linux`，不在 Server crate 里——与 Windows 的 TSF DLL 同一条理由：
**前端不能因为要用协议类型就拖进 Engine 的依赖树**（Server 依赖 core / dictionary / lm / neural，前端一个都不要）。

`scripts/install.sh` 按机器上有哪个框架装对应的前端（`--frontend auto|fcitx5|ibus|both`，缺省 auto = 能装的都装），
`files.py` 只按给的路径照做（清单带 SHA256，装前逐个核对目标不是别人的文件）。三处值得记：
IBus 的组件 XML 与 systemd 单元里都写死了安装后的绝对路径，所以由 install.sh 按 prefix 生成好再交给 `files.py`；**IBus 不读 `~/.local/share/ibus/component/`**，只认 `IBUS_COMPONENT_PATH`，所以另装一个 environment.d 片段把用户目录加在**最前面**——两个目录里有同名组件时以排在前面的为准，装过旧版 deb 的机器上新装的才会生效（这两条都在 IBus 1.5.34 上实测过）；
`XMODIFIERS` 只用来提示「你在用的那个框架这次没装上」，不决定装什么（装完再切框架是常事，两支并存不冲突）；
卸载要先 `systemctl --user disable --now` 再删文件，单元文件没了 systemd 就不认这个名字、停不掉的 Server 会一直占着 socket。

## apps/linux/ibus

第二个前端（IBus），与 `fcitx5` 平级：同一个 Server、同一套协议。按键、焦点、重置与面板三样已经通，
对着真 ibus-daemon 验过（`tests/ibus_engine.rs`）。密码框（`SetContentType`）代码有但没验过。

- `client/`：一条 Unix socket，一问一答。**哪些消息有回包是协议的一部分**——`OpenSession` 与 `LinuxEvent` 有，
  `CloseSession` 与展示回执没有；读多了卡住，读少了下一次答复串位。`Shared` 是 `Arc<Mutex<Option<Connection>>>`：
  锁毒化后取回数据接着用，出错就扔掉连接下次重连（会话要重开）。超时 200 ms，与 Fcitx5 插件一致。
- `key/`：keysym + X11 修饰键掩码 → `KeyEvent`。**这张表与 `apps/linux/fcitx5/src/key/mapping.cpp` 必须一字不差**，
  两个前端喂同一个 Server 同一套 `[shortcut]`，不一致同一个键在两个框架下行为就不同。
  最容易错的是 Shift+数字：X11 给的 keysym 是 `@`，而译词 / 删候选快捷键按物理数字行认，要折回 `0x32`，`character` 仍留 `@`。
- `ibus/`：D-Bus 那一层。`engine.rs` 每个输入上下文一个对象（对象路径当 Server 那边的上下文标识），
  `variant.rs` 拼 IBus 的 GVariant 对象，`view.rs` 把帧折成 preedit / 候选表 / 辅助行三样，
  `service.rs` 找总线地址（`IBUS_ADDRESS` → 按 ibus 的规则算地址文件名 → 扫目录只认 PID 还活着的）。
  **没有「主循环重画」**：Linux Server 不推送（严格一问一答，云联想与神经重排也没接进 Linux），
  每一帧都是某次事件的回包——走进程内引擎那一版的 `active.rs` 与节拍循环因此整块不需要了。
- `UpdatePreeditText` 的参数是 `(vubu)`，**最后那个 `mode` 不能省**：ibus-daemon 解不出来会整条丢掉，
  只在它自己的日志里留一条 CRITICAL，症状是拼音行不显示而这边一点错都看不到。
- `--check nihao`：连 Server、开会话、报能力与焦点、逐键发上去、打印回来的候选；Server 没起着时直接报连不上。
- 端到端测试要等引擎激活再敲键：切引擎之后 daemon 还要走一遍 FocusOut → Enable → FocusIn，
  不等的话第一个字母会被 FocusOut 当「缓冲原样上屏」交出去（真人用的时候引擎早激活好了，撞不上）。

## 独立的选区翻译提供方

Core 的 Engine::set_translation_predictor 与 translation_enabled 将显式选区翻译与普通联想分开；未配置时沿用原提供方，显式配置但不可用时不回落在线服务。手动选区保留首尾空白，私密输入及取消请求使迟到结果失效。

Windows Server 受控退出使用 ExitCode 返回以刷新既有 tracing 队列；启动、退出及脱敏 panic 只记入现有 server 日志，不新增文件或轮转机制。原生异常与强制终止仍可能没有记录。
