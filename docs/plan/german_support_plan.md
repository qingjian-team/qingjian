# 德语学习语言支持 · 设计 note

**日期**：2026-10-08
**状态**：已废弃（SUPERSEDED，2026-10-10）——**本 note 描述的本地三层管线未实施，保留作文档存档。**
**先例**：西班牙语 #48（代码接线）/ #49（数据与生成脚本）/ `b423a4e`（随包）

> **实施情况（以本节为准）**：最终采用方案 D——复用 `tools/gloss-gen` 的 LLM 多语言管线
> （`--languages de`，deepseek-flash）生成 `glossary-de.tsv`，名词冠词经 Wiktionary 快照
> 闸门校验（一致率 98.4%，≥98% 发布），见 #511（核心接线）与 #535（释义数据与生成工具）、
> 讨论见 #509。下文的 official/manual/ai 三层分片、Azure 机翻、`--fragments-dir` /
> `QINGJIAN_DE_FRAGMENTS_DIR`、provenance 与本地 CEFR 等级编排**均未实现、相关脚本不入库**；
> 首版也不随包 `levels-de`（与越南语首版一致）。以下正文为当时的原始方案，未随实施更新。

## 1. Problem Statement & User Value

- **为什么做**：青简的核心价值是「好好输入，顺便多认识一个词」。现有学习语言 en / ja / es，德语是欧标 A1–C1 学习者的缺口；本地已积累一套仅自用授权的德语精修词库资产，可以直接拿来做高频精修层。
- **目标场景**：打中文候选时，右侧一行显示德语译词；名词在精修层带冠词（der/die/das）；设置「统计」页能按 CEFR（A1–C1）看词汇分布。
- **什么都不做的代价**：德语用户只能选英语释义；本地的手编/AI 精修词库（含性别、复数、CEFR）得不到复用。

## 2. User Journey & Core Flow

1. 设置 → 通用 → 学习语言选「德语」（只在装了 `glossary-de` 数据时出现）→ 热加载生效。
2. 打 `kaifa`，首选「开发」右侧显示 `v. entwickeln`；打 `pingguo` 首选「苹果」显示 `n. der Apfel`。
3. 设置 → 统计：词汇（德语）按 A1/A2/B1/B2/C1 分档计数（译词与等级键逐字一致，见 §3 数据口径）。

## 3. 架构与数据模型

### 3.1 代码层（与西语 #48 同构的穷举点）

| 文件 | 改动 |
| --- | --- |
| `crates/qingjian-core/src/candidate/language.rs` | 加 `German` 变体；`code()` 加 `"de"`；`FromStr` 加 `"de" \| "german"`；测试新增 de，并把现有 `rejects_unknown_codes` 用例里的 `"de"` 换成 `"fr"` |
| `crates/qingjian-predict/src/gloss/prompt.rs` | 新增 `GERMAN_SYSTEM_PROMPT`（汉德词典编纂者；名词必须带冠词 der/die/das、动词不定式、每条 ≤3 词）；`system_prompt` 加 German 臂；`clean_text` 加 German 分支：字符集复用西语拉丁扩展区间（含 ä ö ü ß），长度上限 `MAX_GERMAN_CHARS = 40`（复合词），词数 ≤4；补 parse_reply / clean_text 单测 |
| `apps/windows/settings/src/panel/pages/general.rs` | `LANGUAGES` 加 `("德语", "de")`，数组长度 4→5 |
| `apps/windows/settings/src/panel/pages/usage.rs` | `language_name` 加 `Language::German => "德语"` |
| `apps/windows/server/src/assembly/mod.rs` | 仅 `load_vocabulary` 的 levels 循环数组（L174）加 German；释义表装载 code 驱动无需改。**`language_model.rs` 是 BigramModel 文件加载，与 Language 枚举无关，不要改** |
| `apps/macos/src/preferences/controls.rs` | 加 `Language::German => "德语"` |
| `apps/macos/src/host/mod.rs` | `GLOSSARY_LANGUAGES` `[Language; 3]` → 4，尾加 `Language::German` |
| `apps/linux/server/src/assembly/mod.rs` | 仅 levels 循环数组（L140）加 German；glossary 闭包 code 驱动无需改；`language_model.rs` 与本枚举无关，不改 |
| `apps/cli/src/args.rs` / `error.rs` | 帮助文本与合法值报错的语言清单加 de |
| `crates/qingjian-platform/src/config/general.rs` / `mod.rs` | 按西语先例同步清单文案 |
| **协议版本** | `crates/qingjian-platform/src/protocol/mod.rs` `PROTOCOL_VERSION` 7→8。`Translation.language: Language` 随 `Candidate` 走线上帧，加变体属模块注释明列的「必须 +1 并重装 DLL」情形（西语当年没升是历史疏漏，不照抄） |
| `apps/linux/fcitx5/src/qingjian.cpp:159`、`tests/transport.cpp:28`、`tests/support/mock.h:48` | 写死的协议号 7→8（共 3 处） |

Windows 设置页与 mac host 本就「文件不存在的不列」，所以代码可以先于数据合入，无数据时 UI 不出现德语。

### 3.2 数据层（新管线 `tools/corpus/glossary_de.py`，以 `glossary_es.py` 为骨架）

**输入三层，优先级固定为 official > manual > ai**，数据从仓库外的私有分片目录读取（参数 `--fragments-dir` 或环境变量 `QINGJIAN_DE_FRAGMENTS_DIR`，直接指向含分片的数据目录；脚本不内置本机路径），不 vendor 进青简：

1. official：`official_vocab.py::OFFICIAL_VOCAB`，考纲词表 2,732 条（A1/A2/B1，含 gender）；
2. manual：`core_dict.py::CORE_VOCAB_MANUAL`，443 条手编；
3. ai：`core_dict_ext.py::CORE_VOCAB_EXT`，3,968 条 AI 生成；
统一 schema `lemma -> (cefr, pos, gender, plural, definition_zh)`。管线用 `--sources` 控制启用哪些层；本地测试 = 三层全开。

**输出两层合并成 `assets/glossary/glossary-de.tsv`**（格式同现有释义表：UTF-8 无 BOM、LF、按码点排序，走同一套严格解析）：

- **机翻基础层**：复刻西语路线，以 `glossary-en.tsv` 的词表/词性/英文释义为源，Azure Translator 翻德语（密钥仍只从 `AZURE_TRANSLATOR_KEY` / `AZURE_TRANSLATOR_REGION` 读），带断点续跑缓存；**机翻结果一律剥离开头的 der/die/das**（MT 不保证性别正确）。
- **精修覆盖层**：反转三层数据的 `definition_zh`（分隔符 `，；、,;/`，去括号注释），只保留命中 `assets/lexicon/dict.tsv` 的中文键。同一中文键的多个德语 lemma 按「来源优先级（official > manual > ai）→ CEFR 易→难」排序。与机翻层按 sense 槽位合并：精修译词先占槽，不足 2 条时用机翻译词补槽（各槽冠词策略按自身来源，正是方案 (b) 的语义）；精修层给满 2 条则机翻该键不显示。
- 不取 `official_vocab_rich.py`：IPA / 德汉例句不进释义表 schema，本轮无消费方。
- 精修层词形规则：POS 映射 NOUN→n.、VERB→v.、ADJ→adj.、ADV→adv.、PRON→pron.、PREP→prep.、CONJ→conj.、NUM→num.、INTERJ→int.、PART→part.；名词首字母大写；gender 映射冠词并并入译文文本：Masc→der、Fem→die、Neut→das、Plur→die（即 `haus` → `n. das Haus`）；动词 lemma 为不定式原样，连字符形式（`an-sein`）渲染为空格（`an sein`）；无 reading 列（德语拼读规则，与西语同）。
- 同脚本另出 `assets/levels/levels-de.tsv`：`# levels\tA1\tA2\tB1\tB2\tC1`；**键与释义表译词文本逐字一致**（名词含冠词，查表双方都做 lowercase，`LevelTable` 无需改）；只收实际落进 glossary-de 的 lemma（没显示路径的词不进表）。
- 落一份 `data/generated/gloss-de-provenance.jsonl`（gitignored）逐键记录来源层，供抽查与将来生成「无 official」版本。
- 脚本结束打印覆盖报告：机翻条数、精修覆盖条数（分三层）、丢弃条数（中文键不在词库 / 解析失败）。

**本地测试阶段生成物不提交**：`glossary-de.tsv` / `levels-de.tsv` 先留在工作区（CLI 数据发现链 `data/generated → assets/...` 直接可读）；确认效果前不入 git。

### 3.3 许可边界（硬约束）

- manual / ai：可再分发的手编与 AI 离线生成物（ai 层与青简 en/ja 释义同为 AI 离线产物）。
- **official 考纲词表无再分发授权**：仅本地使用，禁止进任何对外提交/PR/安装包。将来要向 upstream 发数据 PR，必须用 `--sources manual,ai` 重生成（provenance 文件可列出被剔除的键），届时 README 署名也不得出现该官方来源。
- 本轮不改打包脚本（`bundle.sh` / `qingjian.iss` / `data-bundle.sh` / `dict-convert pack`）；等数据定稿、随包决策后另起一轮。

## 4. Edge Cases & Resilience

- 一个中文词在同一层有多个德语 lemma：取 CEFR 最易的前 2 条；完全同键重复去重时按精确字形（区分大小写与变音，`schon` 与 `schön` 是两个词）。
- 多义词中文释义串（「出发；发车」）拆出多键，各自挂同一 lemma，属预期。
- 精修层中文键全部不在 dict.tsv 的条目：跳过并计数（实测约 17%~34%，多为生僻释义或拆分失败）。
- 性别为 `Plur` 或缺失：有 Plur 用 die；缺失则不戴冠词（宁可不戴，不可戴错）。
- Azure 调用失败/缺 key：机翻层 fail-loud 报错（同西语脚本）；可用 `--skip-translate` 只跑精修层做离线调试。
- 精修分片目录缺失：精修层跳过并警告，仅出机翻层（冠词随之全无，报告里标明）。
- 老 DLL（v7）配新 Server：协议只警告不阻断，但德语帧会因 unknown variant 整帧失败。本地 CLI 测试不经过 IPC；Windows 真机验证前必须重新编译并重装 TSF DLL（Server 与 DLL 同机同版本）。

## 5. Test Strategy

- **Rust 单测**：language 的 de 解析/code；prompt.rs 德语分支——带冠词名词保留、ä/ö/ü/ß 与长复合词（≤40 字符）放行、超长与非拉丁字符拦截、英文/西语回归不受影响。
- **脚本测试**（仓库无 Python 测试设施，用 stdlib `unittest` 零依赖：`python tools/corpus/test_glossary_de.py`；合成迷你三层词库，不触网）：三层优先级覆盖、中文键反转与命中过滤、POS/冠词/大写/连字符映射、每键 sense 上限 2、levels 键与译词逐字一致。
- **Core 验证（本地测试主手段）**：
  - CLI 位置参数按拼音查询，用拼音而非中文词：`cargo run -p qingjian-cli -- --language de pingguo kaifa fangzi nihao`，人工核对候选旁的 `der Apfel` / `entwickeln` / `das Haus`；
  - 抽查 30 个高频词对照机翻层与精修层差异；直接解析 `levels-de.tsv` 断言等级分布与「每个等级键都能在 glossary-de 的译词里找到」。
- **门禁**：pre-commit（密钥扫描 + fmt + clippy 全 workspace -D warnings）；pre-push 前跑 `cargo test`（非 Apple 平台 `--exclude qingjian-macos`）。
- **真机**（本轮之后）：Windows 重装 Server+DLL，设置选德语，候选窗显示与统计页分档。

## 6. Fog of War & Scope Boundaries

- 无阻塞性未知；Azure 免费层剩余额度、精修层实际观感以本地跑出来的报告为准，不影响方案结构。
- **明确不做**：本期不提 PR；不改三平台打包/安装脚本；不做德语输入方案/键盘；不做德语 reading/读音；不做 emoji-de；不做英文模式的德释；不引入仅本地授权的 official 数据到任何对外产物。

## 实施顺序（批准后进入 plan）

1. 代码接线 + 协议 v8（Rust/C++ 穷举点）→ 门禁全绿；
2. `glossary_de.py` + 离线 unittest；
3. 本地跑全量生成（Azure + 三层精修），CLI 抽查迭代；
4. 同步 `docs/notes/crate-notes.md` 与用户文档措辞（先标注本地数据状态）。
