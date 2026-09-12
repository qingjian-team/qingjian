# TODO

按「从自用到能给别人用」排。勾掉的移到 [roadmap.md](roadmap.md) 对应阶段。`★` 是当前建议的优先项。
2026-09-06 状态：核心输入、翻译（中→英 / 日 24.9 万词、英→中 4.5 万词、注假名、修饰键上屏译词、翻译选中文字）、英文模式候选、
自建词库（基础 8.7 万条 + 随包领域词库 11 本 13 万条，含语料挖出的高频词）、多词库与导入 / 移除 / 开关、偏好设置、`.qj` 容器（启动 50 ms）、
个人 n-gram（二元 + 三元）、词图内敲错边、崩溃不丢、pkg 安装器、README 安装 / 隐私 / 许可都已完成并在自用。
给测试者打包只剩：特殊应用验证（用户自己在各应用里试）；签名等 Developer ID 证书。

## 〇、上线前必须做（不做就不能给别人）

- [ ] 词库质量（产品词库已自建，见 roadmap）：常用词表只有 5.6 万，语料挖出的 1.57 万高频词已并入（蚌埠 / 台州 / 高铁 / 吐槽 这类补上了），
  网络用语目录还空着（`assets/lexicon/04_internet_slang`）；LLM 标注的多音字读音抽查；挖词过滤已进 `dict-convert mine`（2026-09-07，`oov_filter.rs`，与原脚本输出逐行一致）；
  LM 表（TSV 48 MB / `lm.qj` 29 MB）不进 git，先按发布附件对待
- [ ] ★ 首次运行体验：没有 `data/generated/` 时打进包里的样例词库只有几十条，等于不能用；随包数据（词库、LM 表、释义表、emoji）
  要有一套发布版并定好体积（现在 `dict.qj` 3 MB + 领域词库 7 MB + `lm.qj` 29 MB + 释义表 `.qj` 39 MB + 英文词表 2 MB，pkg 34 MB；可再考虑变长编码）；
  发布物 = pkg（`bundle.sh --pkg`），要么 CI 生成数据、要么把 `data/generated/` 当发布附件
- [ ] 仓库瘦身（2026-09-07 决定：主仓库继续单库、官网独立仓库，不拆代码；要处理的是数据）。
  已做：LLM 中间产物 `gloss-llm.jsonl`（30 MB）/ `gloss-en-llm.jsonl` / `pinyin-llm.jsonl` 移到 `data/generated/`（gitignore），仓库只留导出的 `glossary-*.tsv` 与词库源；
  历史在同日压成 `v0.1.0` 单提交（完整开发历史在本地分支 `archive/dev-history`，不推），所以公开仓库里从没出现过这些文件。
  还差：仓库推到 GitHub 后跑一次 `tools/release/data-bundle.sh`，把这三个 JSONL 与 `data/generated/*.qj` 传到 `data` 预发布 Release（现在只在本机与归档分支里有）；
  正式开源后若 clone 太慢，再考虑把 `assets/lexicon` + `assets/glossary` 拆成数据仓库，现在不做
- [ ] ★ 签名与公证：pkg 安装器、自动注册输入源、版本号 / 构建号已做（2026-09-06，`bundle.sh --pkg`）；
  CI 已搭（2026-09-07，`docs/notes/release.md`）：推标签在 macOS runner 打 arm64 + x86_64 两个 pkg、建 Release、生成 `releases.json`，产品数据从 `data` 预发布 Release 下载。
  还差：Apple Developer 账号的 Developer ID 证书（配齐 `release.yml` 头部的七个 Secrets 就自动签名公证）、仓库推上 GitHub 后第一次真跑验证（本机没跑过 workflow）、
  官网改成读 `releases.json`（含 sha256 / 提交 / 构建时间的展示）并配 `QINGJIAN_WEB_TOKEN` 让发版与文档改动自动触发官网构建、更新检查
- [ ] ★ 特殊应用逐个验证（2026-09-07 用户已验：Zed、Terminal 正常；JetBrains 系没装没验）：Secure Input（已有检测，验证密码框不组句、不发云端）、iTerm / Warp、VS Code / Electron、
  浏览器地址栏、沙盒应用（App Store 版备忘录 / 微信）、全屏游戏；每个记录 preedit 模式建议（按应用的开关已有 `[apps]` 分节，
  英文候选按应用关已做，preedit 模式按应用定可以接在同一分节）
- [ ] 登录后不恢复成青简：2026-09-06 两次开机日志都显示登录后当前输入源是系统拼音 / ABC，要手动切回（进程没崩过，panic 与崩溃报告都是零；
  Caps Lock 切 ABC 的系统开关已排除）。怀疑装在 `~/Library/Input Methods` 的 ad-hoc 包每次重装签名变、登录早期枚举没认上；等签名后装 `/Library` 再看，
  或在 `--register` 之外登录时补一次 `TISSelectInputSource`
- [ ] 卸载与重置：卸载脚本已随包（`uninstall.sh`，`--purge` 连数据删，README 有说明，2026-09-06）；还差偏好设置里的「重置学习数据」按钮、
  配置文件版本迁移（新增键有缺省，改名键要迁）
- [x] 许可证定稿（2026-09-07）：GPL-3.0-or-later，`LICENSE` 全文、`Cargo.toml` `license`、「关于」页、README 一致；名字与 logo 不授权。以后引入 JMdict 要补进署名清单


## 一、日常输入体验（决定自己会不会一直用）

- [ ] 整句转换收尾：语料再补（现在是维基 + LCCC 微博对话，新闻 / 小说体还缺；bigram 只留次数 ≥ 6 的前 300 万对，
  `wxqcf` 出 我想去蹭饭 就是「我想 → 去蹭饭」8 次进了表、「我想去 → 吃饭」没进的稀疏问题，简拼把读音放开后这类问题更显眼）；
  个人 n-gram 已做（二元 + 三元），「按应用分别记」看用起来的效果再说
- [ ] 释义兜底收尾：已做（2026-09-06，只对上屏的词库词 / 云端词查，个人释义表 `user-glossary-<语言>.tsv`）；还可以考虑停顿后高亮的候选也查、
  偏好设置里看 / 清个人释义表、导入词库时批量预填
- [ ] 候选窗口里超长释义要截断（CEDICT 表时 你 那条能拉到整屏宽；LLM 表已短，仍要兜底）
- [ ] 翻译选中文字：译文很长时候选窗口的折行（读不到选区的提示已做）
- [ ] 英文模式候选：把数据包的误拼对照表（typos，10 万对）喂进纠正，技术词（kubectl）没有 wordfreq 词频要给底值，两处编辑的纠正
- [ ] 全角 / 半角切换，`。` 与 `．`，`-` `=` 非组句时的行为
- [ ] 快捷短语（`i` 前缀）：编码 → 短语表，用户可增删
- [ ] 问字模式离线版：拆字表（IDS 数据）按部件查字，云端版已有
- [ ] 双拼收尾：词库里 `lue` / `nue` 读音统一成 `lve` / `nve`（双拼 `lt` 解成 lve，`lue` 的词打不出）；菜单栏也给个方案切换
- [ ] 敲错边收尾：敲错四类代价与整段纠错代价 2026-09-12 已在冻结日志（12886 条可评）上扫过，都在峰上，不改（`docs/notes/constant-sweep.md`）；
  个人折扣上限（3）回放从零学分不出好坏，等 `user-typos.tsv` 能带进回放（`--user-dict`）再看；
  退格重打（组句内、跨上屏）已进输入日志 `retype` 事件（2026-09-12），攒够后先按 `docs/plan/model-eval.md` 的尺子算召回与误纠率，再决定喂不喂个人敲错表；
  整段一处编辑那路仍是逐变体跑整句，长句慢（`nihoama` 1 ms、更长的几毫秒）
- [x] 个人 n-gram 插值常数（λ 0.8 / K 8 / 上限 0.5 / D 0.75）2026-09-12 回放扫过：K 全平，λ 0.7–0.75 与 0.8 差 0.3 个点、带神经重排后无差，封顶 0.7 起掉；保留现值（`docs/notes/constant-sweep.md`）。
  「挤掉了」要选四次才翻过「给掉了」是 K 与封顶合起来的设计（一次误选翻不过），不是常数没调
- [x] 词库缺高频「代词 + 的」（我的 / 你的 / 他的）：2026-09-12 加了短语层 `assets/lexicon/phrases.tsv`（`dict-convert phrases`，语料相邻两三词、对话语料里 ≥ 2000 次、
  边界规则，5400 条：我的 / 好的 / 不知道 / 有没有 / 那我），品牌词 `brand.tsv`（青简 210）；词典词头收不到的这一层以后按同一方法补
- [x] 语料里少的领域词（对齐 / 后端 / 词库 / 候选框）：2026-09-12 从日志人工挑 48 条进 `assets/lexicon/domain_words.tsv`，语言模型走合成计数（`docs/notes/domain-words.md`）；
  语料 0 次的（微软拼音 / 悬浮条）按规矩没进，要进得另立白名单
- [ ] 按输入串记的选择只认字面：`wod` 下选的 我的 惠及不到 `wode`；考虑同时按候选全拼记一份、查询取两者最大
- [ ] 已经学进用户词的错读音云端词（`我的 wo di`、`我的哦 wo di e` 这类）没有清理入口：偏好设置词库页给「按读音核对用户词」，或一次性脚本
- [ ] 本地整句模型（已进壳，见 `docs/notes/neural-rescoring.md`）：发版数据包带模型与权重许可；模型后台加载 12 秒看能不能缩（首次 Metal 编译）；
  重排改了切分时应用里的行内拼音要到下一键才更新；日语
- [ ] 个人微调（LoRA 挂在冻结基模上，基模更新后拿本地日志重训）：先在 train 仓库验证收益，再验 candle CPU 训练能不能跑通，都过了才进壳
  （闲时门禁、加载时合并权重、开关与一键清除）；每个模型假设先按 `docs/plan/model-eval.md` 写尺子与基线
- [ ] 日语 emoji 要等有日语输入模式

## 二、产品特色

- [ ] 释义表收尾：覆盖面再扩（次数阈值降到 100 约 10 万词）；JMdict 校对假名与日文词性；等级表（CEFR / JLPT 已随包，见 `assets/levels/`）
  也可做英→中、日→中方向的种子；四六级 / 商务英语词表要先找到许可干净的来源
- [ ] 可选复习（Phase 4）：输入统计、生词识别、词汇统计（含 CEFR / JLPT 等级分布）已做（2026-09-06）；复习容易变成打扰，先不急
- [ ] 云联想收尾（Phase 6）：密钥进钥匙串（等签名定了再做，ad-hoc 签名每次重装都弹授权）、按应用禁用、限流与用量统计
- [ ] 个人模型（Phase 7）：小 Transformer 实验（有评测门槛），见 roadmap

## 三、其他平台

- [~] Windows TSF（Phase 5）：真机自用中，候选窗已覆盖商店 / 任务栏搜索（`uiAccess` + 自签）。
  与 mac 功能齐平尚缺（按价值排；工时=开发+真机验证合计，单人；真机来回是大头）：
  - [~] **① 翻译选中文字 `translate_selection`**（★★★ / 中高 / 1.5–2 天）：**代码完成，待真机测**（2026-09-10）。
    新增 `RequestSelection` / `Selection{text,rect}` 两协议消息；DLL 异步只读会话（`com/selection.rs`，`GetSelection`+`GetText`+`GetTextExt`）
    读选区 → Server `request_translation`（`PredictionKind::Translate`）→ 自绘候选窗显示译文 → 回车 / 空格替换（回 `commit` 走 `InsertTextAtSelection` 替换选区）/ Esc 保留；
    设置页「快捷键」加了翻译选中文字行（改修饰键）。mac 全绿 + windows-gnu 交叉编译 / clippy 过；沉浸式应用读选区、替换选区待真机验。
  - [~] **② 生词「看到轮次」计数 `note_displayed`**（★★☆ / 低 / 0.5 天）：**代码完成，待真机测**（2026-09-10）。
    `dispatch/mod.rs::reconcile_candidates` 里真正 show 那次按当前页调 `Engine::note_displayed`、收窗传空，对齐 macOS `render`；轮次推进、生词满 3 轮毕业。
  - [~] **③ 悬浮状态条**（★★☆ / 中 / 1.5–2 天）：**代码完成，待真机测**（2026-09-10）。
    Server 侧单例，跟候选窗同一条 UI 线程；分层窗合成器从候选窗 `surface.rs` 抽出成 `ui/layered.rs` 两边共用，绘制件（主题 / DPI / 深浅）复用；
    状态条自己一个窗口类 + 窗口过程（`ui/status/`）：`WM_NCHITTEST`→`HTCAPTION` 整块可拖、`WM_MOUSEACTIVATE`→`MA_NOACTIVATE` 不抢焦点、
    `WM_EXITSIZEMOVE` 把位置写回 `[status_bar] x/y`（`GWLP_USERDATA` 存自身指针）。新增协议 `ClientMessage::ModeChanged{english}`（fire-and-forget），
    DLL 在 `update_mode_indicator` 推模式；Server 据此刷、会话关就收起；双拼方案 Server 从 `[general] shuangpin` 知道。配置加 `[status_bar]` 分节（`enabled`/`x`/`y`），
    设置「候选窗口」页加开关。参考水杉 FTB 形态（它用 D2D 且不记位置），我们沿用 GDI 分层窗保持视觉一致并加了记位置。
    mac 全绿（+3 状态条分派测试）+ windows-gnu 交叉编译 / clippy 过；拖动 / 记位置 / DPI / 深色 / 盖高 z-band 待真机验。
  - [~] **④ 删候选屏幕提示**（★☆☆ / 低 / 0.5 天）：**代码完成，待真机测**（2026-09-11）。
    `Frame` 加 `notice: Option<String>`（不参与 `is_empty`），`dispatch/shortcut.rs::forget_on_page` 填、`handle_key` 开头清（只活到下一次按键）；
    自绘候选窗 `ui/candidates/view.rs` 画在拼音行下方（淡色小字）。host 测已加。
  - [~] **⑤ 任务栏点中/英反同步**（★☆☆ / 低 / 0.5 天）：**代码完成，待真机测**（2026-09-11）。
    激活时对转换模式 compartment 挂 `ITfCompartmentEventSink`（`com/conversion.rs`），`OnChange` 读回 `NATIVE` 位、与当前模式不同才翻转
    （防回环），顺带刷指示器 + 上报 Server 让悬浮状态条也同步。纯 DLL 改动、无新协议。
  - [ ] 发版：换 **Certum 开源代码签名证书**重签（开发全程自签 + 本机受信任根，见 `installer/sign-local.ps1`）、
    `windows-v<版本>` 标签与 CI。
  - [ ] **本地整句模型上 Windows**：Server 已接（`dispatch/rescore/`，CPU 推理，设置「云服务」页有开关，安装包带 `data\model`），待真机验：每次重排的耗时（前文 + 几条路径一次前向，CPU 上可能几十到一百多毫秒，超了就缩前文长度）、模型加载时间；
    应用光标前文已接（DLL 起组句时读、密码框跳过），真机看沉浸式应用 / 终端读不读得到；CI 的 windows job 从 `data` Release 取模型三件套（与 mac「数据包带模型」同一件事）。
- [ ] Linux IBus / Fcitx（Phase 5）；配置同步、跨平台词库
