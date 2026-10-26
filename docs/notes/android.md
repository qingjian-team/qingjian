# Android 实现要点

`apps/android` 的功能怎么接到 Core、各处的取舍。目录与构建见 [apps/android/README.md](../../apps/android/README.md)，用户怎么用见 [user/platforms/android.md](../user/platforms/android.md)。

## 键盘

- 布局：拼音 26 键 / 九宫格 / 双拼 / 五笔 86 / 注音大千，加上英文 26 键与日语罗马音，在键盘选择面板里切。英文只切到英文（全键盘），中文布局不变。
- 全键盘：字母键角标（首排数字、二三排符号，中文 H J 是中文引号），长按弹 大写 / 角标符号 / 小写；中文底排 符 / 123，英文底排 !?# / 123（常用符号页）。
  按住空格左右滑挪光标、按住不动进语音（见下文「语音输入」）；回车键按输入框的动作写 发送 / 搜索 / 前往 / 下一项 / 完成（`EnterLabel`），多行框画回车图标。
- 「123」进数字键盘：左列可拖的符号，中间 3×3 数字，右列 删除 / . / @。
- 九宫格是 Core 的 `nine_key` 方案（[design/nine-key.md](../design/nine-key.md)）：左列是同码的拼音，点一下定下音节，不在组句时放标点。
  数字键长按弹 `abc2ABC`（`1` 键是 `1@-_/`），组句中选小写字母就把这一位定成这个字母。九宫格的查询不走模糊音。
- 注音大千按电脑大千键位排四排，键面画注音、进引擎的是大千的 ASCII 键（Core 的 `zhuyin::map_key`）。空格走 `Session::space`：缺声调时是一声，否则上屏首选。
- 双拼九套方案与「输入时显示按键」（`shuangpin_raw_preedit`）同 PC；键上印的声韵母从 Core 的方案表取（`Native.shuangpinKeycaps`），用 `;` 当韵母的方案二排末尾多一个 `;` 键。
- 五笔走 Core 的形码管线，码表是随包的 `assets/schemes/wubi86/table.tsv`（随包时改名 `wubi86.tsv`，第一次切到五笔时读）；「五笔拼音混输」开着时拼音也能打。
- 日语罗马音（测试版）走 Core 的 `RomajiComposer`，不经拼音引擎；候选是小词表汉字、平假名、片假名、原样罗马音，回车上屏平假名。
- 中英键按一下在一对语言间切（缺省 中 ↔ 英），长按竖排弹出全部语言。「换语言时保留中文」开着时另一边总是中文；当前语言与另一边存在设置里。
- 全面屏适配开着时手势条那一截左右各放一个键（切换输入法 / 语音 / 剪贴板 / 收起键盘 / 不放），两边不能一样。
- 密码、网址、邮箱与非文本输入框不过引擎。

## 候选条

- 由 Rust 渲染器（`qingjian-render` 的 `Layout::Strip`）按设置里的主题画成位图贴上（见下文「外观」），与桌面候选窗同一套主题；emoji 渲染器画不出（系统 emoji 是 COLRv1），壳在词的位置上补画。
  真机结果见 [design/rendering.md](../design/rendering.md)「Android 候选条」。
- 长按候选弹气泡：左右排开译词，滑到哪条抬手上屏，对应桌面端修饰键 + 数字。删得掉的候选（Core `CandidateKind::forgettable`）末尾多一项「删除」（`Session::forget`，对应桌面端 ⇧ + 数字），结果用 toast 提示；
  没有译文时气泡里只有「删除」，不预选，得往上滑进气泡才算，免得长按松手误删。
- 右端 ⌄ 展开全部候选：盖住键盘，按字宽排、每行撑满、上下滑，右列 返回 / 删除 / 重输。
- 分次选词留在组句里（Core 的 `hold_selections`，键盘设置开关，缺省开）：选词没吃完拼音时 `commit` 回空串，留着的词经 `Query::marked_text` 显示在拼音前面，退格由 Core 撤回、回车 `take_raw` 带上留着的词。
  空格在有候选时只选首选（留着就接着选），没有候选才原样上屏；标点、符号这类「顶首选」要结束组句，壳选首选后若还在组句就 `commitRaw`，留着的词连同剩下的拼音一起落下。
- 不在组句时光标前是「用户名@」，候选条列常见邮箱后缀（Core 的 `email_completions`，经 `Session::complete`）；中文组句中按 @，敲的字母原样落下当用户名。
- 光标前末尾是一道算式（数字键盘上直接上屏的 `3+4`、`3×4=`）时，同一条路给结果（Core 的 `calculation_completions`）：帧的 `replace` 是上屏前要删掉的字符数（换成结果那条），壳 `deleteSurroundingText` 后再上屏。壳只在光标前最后一个字是数字、`)` 或 `=` 时才问。

## 快捷输入

- `rq` / `sj` / `xq`、`v1+2`、`u4e00` 都是 Core 的快捷候选。手机上数字与运算符不在字母键盘上：帧带 `expression`（`Engine::expression_mode` 或 `unicode_entry`），为真时「123」「符号页」不顶首选，数字键盘上的数字与 `+ - * / ( ) . ^` 进缓冲区（× ÷ 换成 * /），`=` 上屏「算式=结果」那条。
- 九宫格是数字串，`77` 不认 `rq`；拼音列或长按把字母都定下来（缓冲区成了 `r'q`）才出。全拼出了「日期 / 时间 / 星期」时 Core 把同一组写法插在这个词后面、它的 emoji 之前（九宫格 `7474` 的「日期」常排第四，所以看前五个词）。

## 输入设置

键盘设置「输入」一节与 PC 的同名项对应，整份 `InputOptions` 经 `Session::set_options` 交给 Core，名字同 PC 配置：

- 模糊音：`[fuzzy]` 九条。
- 中文用全角标点：中文键盘的逗号句号与拼音列 ，。？！： 键带着对应的半角字符，上屏前经 `Engine::punctuate` 转换（引号成对、数字后的点保持半角）；关了出半角并 `note_passthrough`。
  符号页里的标点是明确挑的字符，不转换。
- 打拼音时也给英文词 / 中文排在英文词前面：`english_in_chinese` / `chinese_first`。
- 英文键盘给候选：关了英文字母直接上屏，壳里判断，Core 没有对应项。
- 学习输入习惯：`learning`，即 `Engine::set_learning`；私密输入另走 `Session::set_private`，由输入框的 `IME_FLAG_NO_PERSONALIZED_LEARNING` 决定（浏览器无痕标签页会带），对应 PC 上应用声明的私密输入框。Core 的私密输入不学习、不记生词与例句、不发云端，输入统计照记（同 PC）。
- 繁体字形：`traditional_variant`（`taiwan` / `hongkong` / `standard`）。功能面板的「繁体」仍是开关，开着时按这里的字形出（`Session::set_traditional` 用会话记着的字形）。
- 附加词库：`domains` / `disabled_dictionaries`，同 PC 的 `[dictionaries]`。会话每次交设置时按开关列出该挂的文件（路径、修改时间、大小），与上次一样就不动，变了重新 `set_extra_dictionaries`。
- 学习语言：`learning_language`（`off` 不显示译文）。装配时不装学习语言的释义表，壳交设置时 `Session::set_options` 按语言装（随包表叠个人释义表），语言变了才换，换前先 `flush_learning`。
  几门语言的释义表全部随包（`glossary-*.qj` 共约 65 MB），离线即换；等级表 `levels-en.tsv` / `levels-ja.tsv` 也随包。

PC 上只对实体键盘有意义的项（Shift + 字母、切换键、翻页键、高亮移动键、状态条、标点先上屏候选的开关）不搬到手机。

## 面板

面板盖住候选条与键盘那一整块，高度跟键盘走：功能面板（常用功能宫格 + 就地改的开关，「繁体」在这里，字形按键盘设置里选的）、键盘选择、分类符号、表情（Emoji / 颜文字，能接肤色的排出五档滑选）。
符号与表情的内容是 Core 的面板目录（`Native.panelCatalog`），与桌面端符号面板同一份；「常用」按用过的排（Core 的 `common_items` + `PanelUsage`），每次打开取一次。

## 词库

- 领域词库是 `data/generated/dicts/` 的 11 本 `.qj`（约 7.5 MB），`stageData` 放进 `assets/data/dicts/`，`BundledData` 递归解到 `files/data/dicts/`；缺省只开成语，同 PC。
- 导入走 SAF（`ACTION_OPEN_DOCUMENT`，`*/*` 多选：TSV 与 Rime 文件的 MIME 各家文件管理器给得不一样）。壳按原文件名拷到缓存目录，Rust 的 `store::dictionaries` 交 `qingjian_dictionary::import` 转成 `.qj` 放进 `files/user/dicts/`，再删掉拷贝。导入与删除后改设置里的 `dictionaries_stamp`，开着的键盘重建、重交选项、会话按文件重挂。
- 「删除」直接删文件：PC 的「移除」是挪进 `removed/`，手机上用户碰不到这个目录，挪过去等于留垃圾。
- 有的厂商文件选择器按自己的文件索引列文件，`adb push` 进去的要先发 `MEDIA_SCANNER_SCAN_FILE` 广播才搜得到。

## 剪贴板与常用语

- 系统剪贴板复制了文字或图片由 `ClipboardWatcher` 记进历史：默认输入法读剪贴板不要权限；标了敏感的不记；可关、可设最多 100 / 500 / 1000 / 5000 条。
  存取在 Rust 侧 `qingjian_learning::ClipboardHistory`（`clipboard.jsonl`，图片另存 `clipboard-images/`）。
- 像代码的文字（Core `code::looks_like_code`）用等宽字体原样显示，「代码高亮」按 Core `code::highlight` 上色。
- 图片走 `commitContent` 发出去（`ClipboardImageProvider` 只读临时授权，`query` 要给 `OpenableColumns`，否则有的应用把 content:// 地址当文字插进去）；输入框不收图片就放回系统剪贴板。
- 拆词（Core `text_pieces` 切块）可点选或横滑连选，选着的实时放进输入框、处在编辑中，返回或完成确认上屏。
- 常用语（`PhraseBook`，`phrases.json`）按分组列，卡片带输入码；带输入码的挂成 Core 自定义短语，打码直接出。输入码最多 32 个字母，跟 Core 自定义短语的上限一致。

## 翻译选中的文字

- PC 的「翻译选中文字」（快捷键读选区）在手机上改成系统文字选择菜单里的「青简翻译」：`ProcessTextActivity` 接 `ACTION_PROCESS_TEXT`，透明窗口 + 底部卡片（主题 `Theme.Qingjian.Sheet`），不进最近任务。
  原应用没给 `EXTRA_PROCESS_TEXT_READONLY` 时多一个「替换」，译文经 `setResult` 的 `EXTRA_PROCESS_TEXT` 交回去。
- 翻译走 `Engine::request_translation`，方向由 Core 按文字判断。Activity 不借输入法的引擎（输入法不一定是青简，也不该打断它的会话），另装一个轻量 Engine（`assemble_translation`：主词库 mmap + 学习语言释义表，几毫秒），用同一套 `setCloud` / `translate` / `pollTranslation` 出口。
- 失败（出错、回包解析不了、模型没给内容）由 `qingjian-predict` 的 worker 回一条空结果，壳当场显示红字「翻译失败」并只给「重试」，与译文分开，不必等超时；快捷翻译条的失败状态同样是红字。
- 自家 App 的输入框要列出文字处理项得在清单声明 `<queries>`（Android 11 起的包可见性）。有的厂商系统在青简试打框的选择菜单里只列「搜索 / 分享」，别的翻译应用也不出，真机上用 `am start -a android.intent.action.PROCESS_TEXT` 验的。

## 浮动编辑条与快捷翻译

- 常用语编辑框与快捷翻译条都是 `FloatingEditor`：开着时 Composer 的输入连接换成它的，键盘打的字进编辑条；打输入码时换英文全键盘。
  编辑条里打的字不进应用，开着时 `Engine::set_metered(false)`，不计今日字数。
- 快捷翻译：原文（不含组句中的拼音）停顿 450 ms 后经 `Engine::request_translation` 发云端（汉字为主译成学习语言、外文译成中文），每 100 ms 取一次；
  译文用组合文字放进外面的输入框，改原文跟着换，确定或回车落定，取消撤掉。
- 云端是 `qingjian-predict` 的 `CloudPredictor`，开 `rustls` feature（ring + `webpki-roots`，不用平台证书库）；云联想关着时配置只开翻译，平时打字不联网。
  `rustls` 的 crypto provider 要在 async-openai 建客户端之前装好，见 [crate-notes.md](crate-notes.md) 的 predict 一节。不记接口用量：用量文件靠文件锁合并，安卓上 `lock()` 不支持。
- 密钥由 `SecretStore` 用 Android Keystore 里生成的 AES-256 钥匙（不可导出、卸载即毁）做 AES-GCM 加密后只存密文；明文只在调 JNI 时临时拼进 JSON，Rust 侧不派生 `Debug`、不进日志。
  应用关了系统备份；快捷翻译、翻译选中的文字、释义兜底与云联想要联网（`INTERNET` 权限），「只在 Wi-Fi 下使用」读网络类型要 `ACCESS_NETWORK_STATE`。
- 释义兜底（键盘设置「云端大模型」里的开关，缺省关，同 PC 的 `CloudGlossFiller`）：会话 `set_gloss_fill` 用同一份云端配置起 `CloudGlossFiller`，Core 在上屏时判断「释义表查不到、不是私密输入」才发那个词；
  `Handle::operate` 每次操作前 `poll_glosses`，补好的释义当场进个人释义表（`user-glossary-<语言>.tsv`），下一帧就有译词。
  PC 上它随云联想一起开，手机上单独一个开关；PC 的「生词本整句翻译」（`vocabulary_translation`）会发整句，手机不开。
  服务在键盘重建、引擎装好、云端配置（`cloud` 那份 prefs）改了时同步；`Engine.setGlossFill` 按配置摘要去重，同一份不重交（每交一次 Rust 侧都另起一条线程）。
  云端没填好时设置页不放开关，换成一行说明。

## 云联想与整句补全

- 键盘设置「云端大模型」里的开关，缺省关，同 PC 的自动联想：`CloudSettings.predict` 打开 `PredictConfig` 的 `automatic` 与 `sentence`，两个云端词槽，停键 500 ms，两次请求至少隔 1 s，超时 10 s；生词本整句翻译不开。
- 会话 `refresh` 每次查完候选就 `request_prediction`（防抖在 worker 里），壳在一段组句的第一帧之后读光标前 64、后 32 个字交 `setSurrounding`，交了按带上下文的再要一次。
  组句中 Composer 每 150 ms `pollPrediction`，结果插在本地首选之后：先整句补全，再云端词，与本地重复的不要；选中整句走 `accept_prediction`。
- 九宫格敲的是数字，Core 用这次查询的拼音行（`QuerySnapshot.pinyin`，九宫格时取 `typed_display`）问云端，没查过这段就不发。
- 候选条的 `strip_cell` 给 `cloud` 候选前加小云朵、字用 `@cloud` 色（同 PC 候选窗）；展开的网格里同样画云朵（`PanelIcons.Icon.CLOUD`），字用 `Palette.cloud`。
- 「只在 Wi-Fi 下使用」缺省开：服务 `syncCloud` 在键盘建起、每次开始输入和云端配置改了时看 `Network.onWifi`（Wi-Fi 或有线，开 VPN 也认得出），不在就退回只翻译。
  打字途中换网络要到下次开始输入才跟上。快捷翻译与释义兜底不看这个开关。
- 工具框（快捷翻译、常用语编辑）开着时会话不发、不收联想（`set_metered(false)` 时 `tools_open`），快捷翻译的译文不被组句联想作废。

## App

启动页底部 tab 切 首页 / 生词本 / 我的，配色 `AppColors` 与键盘分开。首页是没切到青简时的切换卡、输入统计看板（`Native.statsDetail` 传 `off` 不读生词本：今天 / 近 7 天 / 累计、最近 30 天柱状图、四个比例）、常用功能与试打框；
二级页「键盘设置」「输入统计」（点看板进，「我的」里不再单列入口）「关于」。「我的」页的累计卡仍用 `Native.usageStats`。

- 生词本页（`Native.vocabulary`）一次交 `{language, filter, level, change}`：Rust 侧先改（标认识 / 删除，写回 `user-vocab.tsv`）再按筛选与等级列，最多 500 条，另给符合的总数。
  输入法进程里的生词本落盘前会合并文件（`VocabularyBook::refresh`），App 这边的改动不会被盖掉。搜索同 PC（`search`：不分大小写，词、带它上屏的中文、例句里有就算），在 Rust 侧筛；搜索框常驻，打字停 250 ms 后只重填下面的词表，不丢焦点。PC 生词本的时间线、导出不做。
- 「看到」次数：候选条每画一帧，`renderStrip` 把视口里（不算 ⌄ 那一格）看得见的候选与译文交给 `Engine::note_displayed`，横滑后才露出来的不算，同桌面端只算当前一页。
- 输入统计（`Native.statsDetail`）的派生数与 PC 设置程序统计页调同一套函数（`qingjian_learning::figures`，见 crate-notes 的 `UsageStats`），两端「近 7 天」一致；PC 的一年日历改成最近 30 天柱状图，云服务用量不做（用量文件靠文件锁，安卓上不支持）。

## 高级

PC 设置程序的「高级」页（配置文件、日志、输入日志）在手机上改成「我的 → 高级」，只有导出数据与清空学习数据，不露文件路径；配置文件编辑与输入日志开关不做。

- 导出：`ACTION_CREATE_DOCUMENT`（`application/zip`）让用户选位置，壳用 `ZipOutputStream` 把文件平铺写进去；打包哪些由 Rust 的 `store::learning::export_files` 定：
  学习数据目录顶层的 `.tsv` 与 `.json`（输入习惯各表、生词本与时间线、输入统计、常用语、面板常用、个人释义表）。
  剪贴板历史可能有密码、验证码，不带（同 PC 导出日志时抹掉密钥）；导入的词库与设置也不带。导出前 `Engine.flushAll` 让开着的引擎落盘。
- 清空：先勾输入习惯（`FrequencyLearner::files` 列出的词频与各侧车表）和 / 或生词本（`user-vocab.tsv` 与 `vocab-log.tsv`，全部语言，同 PC 生词本页的「清空」），再确认一次。
  输入统计、常用语、剪贴板、词库不动（PC 也没有清它们的入口）。
- App 与输入法同进程，引擎内存里的学习数据下次落盘会把删掉的写回去，所以 `Engine.clearLearning` 先把进程里开着的引擎（`Engine` 伴生对象里登记）落盘卸下，删文件，再重新装配；
  装配到一半被卸下的那次结果按 `generation` 作废。重装好之前按键原样上屏，`onEngineReady` 重交方案与选项。

## 外观

「我的 → 外观」对应 PC 设置程序的「外观」页，只搬主题、深浅色与字号；拼音显示位置、渲染引擎、字体、排布、过渡动画是桌面候选窗的事，不做（字体以后有人要再说）。

- 主题：列的是 `qingjian-render` 的全部内置主题（`Native.themes`），与 PC 同一份，PC 新加主题手机自动跟上；存 `theme`（同 PC 的 `[general] theme`）。卡片上的候选条是渲染器用样例帧画的（`appearance::preview`，`Native.themePreview`），下面几排键按该主题的键盘配色画。
  预览在 App 进程里画，渲染器不能跨线程，放在 `thread_local` 里，壳只在主线程调，画过的按（主题, 深浅, 倍数, 字号档）缓存。
- 键盘配色：`Palette` 不写颜色，整份由 Rust 推（`qingjian_render::KeyboardColors`，规则见 [design/theme.md](../design/theme.md)「手机键盘」），`Native.keyboardPalette` 交 JSON，壳按（主题, 深浅）缓存。
  键盘、面板、浮动编辑条、长按气泡都吃这一份；候选条由会话的画笔按同一主题画（`Native.setAppearance`，引擎重装后 `Engine` 记着重交）。
- 深浅色：`appearance` 三选一（跟随系统 / 浅色 / 深色，同 PC），跟随系统时看 `uiMode`，系统切换时服务的 `onConfigurationChanged` 重建键盘。锁定浅色的主题（四个角色主题）以主题为准，`Palette.dark` 用 Rust 判断后的结果，候选条也按它画。
- 字号：四档（小 0.88 / 标准 / 大 1.15 / 特大 1.3），对应 PC 的候选字号与译文字号，手机上合成一项、按比例调（用户选的）。Rust 侧把主题的 `candidate` / `annotation` 字号乘上倍数交 `Theme::with_text_sizes`；
  候选条在样式上写死的 19 点跟着缩放（渲染器的 `font_ref` 改动），译文截断宽度同比放大；条高按 `max(1, 倍数)` 放大，展开的候选网格同比缩放。键面上的字不跟（用户选的）。

## 语音输入

设计见 [design/voice-and-swipe.md](../design/voice-and-swipe.md)。现在的识别方是 Rust 侧的火山引擎豆包流式识别（`crates/qingjian-voice`）；「语音识别」页没配好时退回占位的 `UnavailableRecognizer`，三处入口都提示「还不能用」。

- 入口都进 `voice/VoiceInput`：`KeyboardView` 按住空格 400 ms 不动问 `onVoiceHold`（开着「长按空格说话」时），开不起来退回挪光标；横向滑开一格（14 dp）先到就是挪光标。工具栏话筒与全面屏底栏语音键是点按进。
- 识别方式是 `VoiceRecognizer`（是否可用、要不要录音权限、开始、说完、取消，回调中间结果、最终结果、出错、音量）。现有 Rust 侧识别方 `EngineRecognizer`：`AudioRecord`（`VOICE_RECOGNITION` 源）录 16 kHz 单声道，每 100 ms 一截经 `Native.voiceFeed` 交 Rust、每 100 ms `pollVoice` 取结果；`voiceAvailable` 为真（配好了）时 `VoiceInput` 才选它。收尾（说完、出错、取消）一律在 `stop()` 里停录音线程，出错时也得停，不然麦克风一直开着。以后可加系统 `SpeechRecognizer`、切到系统语音输入法。
- 中间结果用 `setComposingText` 放进输入框（走 `Composer.Host.connection`，快捷翻译、常用语编辑条开着时进编辑条）；最终结果经 `Session::commit_voice` → `Engine::commit_voice` 上屏，替换编辑中的那段。开始前组句先落下（同回车）。
- `VoicePanel`（`PanelKind.VOICE`）由 `InputPanel.openVoice` 盖上，不经 `openPanel`；返回键、换输入框、收起键盘、键盘重建都取消这一段。两块弧是面板下方一个大圆（半径 = 面板宽）顶上离正上方 4°–28° 的两段，按着空格时 `KeyboardView` 把手指的屏幕坐标交 `VoicePanel.track` 判落在哪块（判定比画出来的宽一些），松手时按它撤回、上屏并执行动作或只上屏。右块写 `enterLabel`（快捷翻译条开着时用它的），执行动作走 `sendDefaultEditorAction`；多行框写「完成」。
- 录音权限：`MicrophonePermissionActivity` 透明窗口代为 `requestPermissions`（自己的任务、不进最近任务，免得把 App 带到前台），结果回调给入口，给了直接开始。拒绝了入口提示没有权限。
- 配置：「我的 → 语音识别」（`VoiceSettingsActivity`）与键盘设置里的云端大模型分开，存在 `voice` 那份 prefs（`VoiceConfig`：服务、资源 ID、密钥密文）；密钥同样经 `SecretStore.seal` 用 Keystore 那把钥匙加密。服务监听这份 prefs，`Engine.setVoice` 交 `{provider, api_key, resource_id}` 的 JSON 给 `Session::set_voice`（同一份不重交），「测试连接」走 `Native.voiceTest`（识别半秒静音，阻塞，后台线程调）。

## 滑行输入

设计见 [design/voice-and-swipe.md](../design/voice-and-swipe.md)「滑行」。解码在 Core（`ShapeDecoder` 按词表打分），壳只判定、记点、画线。`Session` 按在打什么给候选词：中文是主词库的音节串、英文模式与 `decodeSwipe`（字母直接上屏）是英文词表，两份都在第一次滑行时建（`OnceLock`）。

- `KeyboardView.swipeLayout` 由服务在 `showKeyboard` 里按当前键盘设（字母页，且是拼音全键盘、英文键盘或常用语编辑条的英文键盘）；还要设置里 `swipe_input` 开着。
- `SwipeTrail` 管一笔：第一根手指落在 `Key.Letter` 上就 `begin`，`ACTION_MOVE` 把历史点一起记上（快划时两帧之间隔得远，不记历史点拐角会被抹平），离起点 0.6 键宽转成滑行，这时撤掉这根手指的按下态（不画气泡）与长按计时。
- 抬手时 `toJson(slots)` 拼 `{points, keys}`（像素；键是所有字母键的中心与宽高），`Composer.onSwipe`：组句或该组句时 `Native.swipe` → `Session::swipe`（字母逐个 `engine.push`，最后刷一帧），英文直接上屏时 `Native.decodeSwipe` 拿字母串上屏，Shift 亮着首字母大写。
- 真机用 `adb shell input motionevent DOWN / MOVE / UP` 模拟（同一条 `adb shell` 里串起来，不然每步间隔太长会先触发长按）。

## 还没做

本机离线语音识别、神经重排、用户主题（PC 主题目录里自己写的主题）。
