# ADR-0002 配置模型归一

- 状态：已接受（Accepted）
- 日期：2026-11-20
- 适用范围：用户配置文件（`config.toml`）的 Rust 模型、序列化/反序列化、默认值与迁移、校验与错误报告，以及设置程序对配置的消费方式
- 相关 ADR：[0001-dependency-direction.md](0001-dependency-direction.md)（配置类型归属 L3 平台适配层）、[0003-error-model.md](0003-error-model.md)（配置错误的分类与边界降级）

## 1. 背景

### 1.1 现状结构（实测，非估算）

配置模型集中在 `crates/qingjian-platform/src/config/`，顶层入口是 `Config`：

- `crates/qingjian-platform/src/config/mod.rs`：729 行；`Config` 结构体在 91–135 行，共 **14 个字段**：`auto_phrase`、`general`、`custom_phrases`、`shortcut`、`fuzzy`、`dictionaries`、`aux_code`、`apps`、`predict`、`status_bar`、`model`、`clipboard`、`update`、`code`（`mod.rs:95-134`）。
- `crates/qingjian-platform/src/config/general/mod.rs`：541 行；`GeneralConfig` 在 33–254 行，共 **64 个字段**，覆盖学习语言、候选排布、外观字体、英文候选、标点、方案、双拼、辅码、繁体、界面语言、日志与学习开关等多个域。该 struct 已登记在质量门豁免里（`tools/quality/allowlist.toml:49`，阈值 struct_fields = 40，见 `tools/quality/check_code_quality.py:204`）。
- `crates/qingjian-platform/src/config/shortcut.rs`：260 行；`ShortcutConfig` 在 14–76 行，共 **18 个字段**；其中 `mode: ModeKeys` 用 `#[serde(flatten)]` 平铺到 `[shortcut]` 分节（`shortcut.rs:16-17`）。
- `crates/qingjian-platform/src/protocol/server.rs`：`InputSettings` 在 16–67 行，共 **12 个字段**。它是 Server 下发给 TSF DLL 的「按键行为」协议子集，不是用户配置模型（`server.rs:9-14` 的注释明确：DLL 不读配置文件）。
- `crates/qingjian-predict/src/config.rs`：`PredictConfig` 在 7–54 行，共 **15 个字段**，定义在 predict crate 内，由 `Config.predict` 嵌入（`mod.rs:119`）。
- 其余子结构字段数实测：`AppsConfig`（`config/apps.rs:124`，2 字段）、`DictionariesConfig`（`config/dictionaries.rs:10`，4）、`AuxCodeConfig`（`config/aux_code.rs:12`，3）、`AutoPhraseConfig`（`config/auto_phrase.rs:7`，2）、`ClipboardConfig`（`config/clipboard/mod.rs:19`，8）、`LocalModelConfig`（`config/model/mod.rs:15`，4）、`UpdateConfig`（`config/update/mod.rs:12`，2）、`StatusBarConfig`（`config/status_bar.rs:7`，4）。

### 1.2 读写与迁移现状

- **加载**：`Config::load`（`mod.rs:540-564`）：文件不存在按默认值；存在但解析失败返回 `ConfigError::Parse`，不静默吞掉笔误。加载后登记密钥掩码并设置界面语言（`mod.rs:557-562`）。
- **写入**：`set_value`（`mod.rs:574-604`）、`set_array`（`mod.rs:608-634`）、`write_template_if_missing`（`mod.rs:637-643`）。写入走 `toml_edit::DocumentMut`，保留注释与键顺序；先写临时文件再改名，不留半个配置文件（`mod.rs:602-603`、`write_file` 715-726）。文件有语法错误时报错不写，不替用户「修复」成丢注释的文件（`mod.rs:571-573` 注释）。
- **旧键双读**：`GeneralConfig` 保留了若干 `Option` 旧键字段，新键优先、访问器合并旧值：
  - 方案轴：新键 `scheme`（`general/mod.rs:185`），旧键 `shuangpin`（219）、`zhuyin`（222），合并逻辑在 `scheme()`（386-423）；
  - 形码轴：新键 `code`（212），旧键 `wubi`（215），合并在 `code()`（462-472）；
  - 繁体：新键 `traditional_variant`（229），旧键 `traditional`（233），合并在 `traditional()`（344-354）；
  - 外观：`appearance`（53）与旧写法 `theme` 外观词互认，逻辑在 `appearance()`（369-373）、`theme_id()`（376-383）。
- **取值级迁移**：`rewrite_retired_values`（`config/retired.rs:14-38`）只由设置程序启动时调用，把已下线取值（`model.name = "zhiwei"`）改成现取值；输入法进程只读不写配置。
- **单项容错**：不认识的枚举/字符串取值只让这一项回缺省并记 warn，不让整份解析失败（`traditional_from_key`，`general/mod.rs:530-538`；未知拼音方案回全拼，392-396）。自定义短语写错的条目只跳过它自己（`mod.rs:100-101` 的自定义反序列化）。
- **回归基线**：`crates/qingjian-platform/tests/config_history/` 下保存了 v0.1.0 起各平台、各版本的真实配置样本；测试 `samples_are_present`、`every_released_template_loads`、`traditional_edits_from_old_settings_still_apply`（`tests/config_history.rs:39,49,63`）保证老样本可读。仓内测试还包括 `template_parses_to_defaults`、`partial_file_keeps_other_defaults` 等（`config/tests.rs:6,12`）。

### 1.3 UI 消费现状

- 共用设置程序 `apps/settings`：`Settings` 持有 `config: Config`（`apps/settings/src/settings/mod.rs:51`）与 `config_error: Option<String>`（`mod.rs:77`）；读写只经 `Config::set_value` / `Config::set_array`（`apps/settings/src/settings/config.rs:9,25`），重读失败时记日志并把错误字符串存进 `config_error`（`config.rs:33-41`，字符串化在 40 行）。
- Windows 旧版设置 `apps/windows/settings`：`Config::load(&path).unwrap_or_default()` 起步（`panel/component.rs:29`），页面直接读 `self.config.shortcut`、`dictionaries`、`aux_code`、`update` 等公开字段（`panel/component.rs:123,222,241,276,281,291,303,329,394`）。
- 导入导出：`export_settings` / `import_settings`（`config/transfer.rs:21,36`）；本机项（`predict.api_key`、`status_bar.x/y`）导出时剔除（`transfer.rs:11-15`）；导入前先当配置读一遍，读不通不动现有文件（`transfer.rs:36-39`）。

## 2. 决策

### 2.1 配置事实源与归属

1. 用户配置始终是**单份 TOML 文件**，菜单、设置窗口、手改文件三个入口都只写这一份文件，再由壳热加载（沿用 `mod.rs:88-90` 的既定原则）。
2. 配置模型类型归 L3 平台适配层（`qingjian-platform`）；`PredictConfig` 这类随能力 crate 自带的子配置允许留在原 crate，由 `Config` 嵌入，现状即如此。L1/L2 不得新增用户配置文件结构（见 ADR-0001 D1）。
3. `InputSettings` 这类跨进程协议下发类型不属于配置模型：它是 Server 按会话算好的行为子集，不参与配置文件的序列化往返，也不从 `Config` 派生嵌套结构。

### 2.2 按域归子结构的边界（规则层）

`GeneralConfig` 的 64 个字段按下面的域归组；域是拆分时的归属依据，不是要求一次拆完：

| 域 | 覆盖字段（现状） | 归属判断 |
|---|---|---|
| 方案与编码 | `scheme`、`code`、`shuangpin_raw_preedit/window/full_pinyin`、`key_hint*`、`aux_code_*`、`shift_letter`、`numpad_digit` | 决定怎么把按键读成编码 |
| 候选与排布 | `page_size`、`page_keys`、`layout`、`horizontal_grid`、`grid_page_keys`、`vertical_reverse_above`、`preedit`、`hold_selections` | 候选窗怎么排、怎么翻 |
| 外观与字体 | `appearance`、`theme`、`renderer`、`font`、`candidate/annotation_font_size`、`candidate_font_weight`、`animations` | 画出来什么样；与 render 层主题对接 |
| 英文与纠错 | `english_candidates`、`chinese_first`、`spelling_correction`、`english_in_chinese`、`english_max_in_chinese`、`english_min_position`、`show_english_pronunciation`、`english_mode`、`default_english`、`remember_mode_per_app` | 英文候选与中英模式行为 |
| 标点与第三输入法 | `full_width_punctuation`、`english_full_width_punctuation`、`pair_punctuation`、`english_pair_punctuation`、`half_width_symbols`、`punct_commits`、`double_shift_input` | 标点转换与模式切换 |
| 译文与学习 | `learning_language`、`translation_pos`、`show_tones`、`emoji`、`kaomoji`、`symbols`、`auto_space`、`learning`、`system_text_replacements` | 候选注记与学习开关 |
| 繁体与界面 | `traditional_variant`、`traditional`、`last_traditional`、`ui_language`、`log_level`、`input_log` | 字形、界面语言、诊断 |

**结构拆分的代码执行在独立工作线 `refactor/split-others`（独立工作树）交付；本 ADR 只定边界、命名、兼容与校验规则，不规定文件怎么拆、字段怎么搬。** 主题侧 `crates/qingjian-render/src/theme/mod.rs`（791 行）的拆分同属那条工作线，本文不重复其方案。

### 2.3 序列化与键名兼容

1. **公开 TOML 键名冻结**：用户文件里的 `[section] key` 是对外契约，不随 Rust 结构重组而改名。Rust 字段需要重命名时，用 `#[serde(rename = "...")]` 对齐旧键，不改文件写法。
2. **缺字段走默认**：所有配置子结构保留 `#[serde(default)]` 或显式 `Default`；一份只写了部分键的老文件必须解析成功，未写项取缺省（基线测试 `partial_file_keeps_other_defaults`）。
3. **旧键双读、新键优先**：需要换键名时，旧键以 `Option` 字段保留反序列化，新键有值就不看旧键；访问器合并两套值（沿用 `traditional_variant`/`traditional`、`scheme`/`shuangpin`/`zhuyin` 的现成模式）。写入路径只写新键，不把旧键复活进文件。
4. **`#[serde(flatten)]` 克制使用**：现状仅 `ModeKeys` 一处 flatten（`shortcut.rs:16`）。新增子结构默认用嵌套表或具名字段；再引入 flatten 必须先评估键名冲突、序列化往返与模板注释位置，并用往返测试守住。
5. **嵌套表改路径需双读**：兼容期内不允许把现有扁平键挪进新嵌套表而不保留旧路径的反序列化；确需改路径时，按 2.3-3 的双读规则走，读到旧路径值时在内存里归并。
6. **未知取值单项回退**：不认识的枚举、方案名、主题 id 等，只让这一项回缺省并记 warn，整份配置照常加载（现状模式见 1.2）。

### 2.4 默认值、迁移与文件完整性

1. 默认值集中在各子结构的 `Default` 实现里，不散落在读取处；值域清洗（clamp、非法字符回退缺省）由访问器负责，现状示例：`page_size`（`general/mod.rs:496-498`）、`english_max_in_chinese`（501-504）、`english_min_position`（507-509）、`key_hint_opacity`（434-436）、`key_hint_size`（439-441）、`aux_code_key`（480-486）、`page_keys`（512-526）、`candidate_font_weight`（489-493）。
2. **老配置文件必须原样可读**：不要求用户迁移文件才能启动；解析得了就读，读不了按文件级错误处理（见 2.5）。
3. **不静默改写坏文件**：语法错误的文件报错、不写；就地改写只针对已下线取值，且由设置程序触发，输入法进程不回写用户文件（`retired.rs:1` 的分工）。
4. 配置写入保持原子性与注释/顺序保留，继续走 `toml_edit` + 临时文件改名，不退回整体序列化重写文件。

### 2.5 校验与错误报告

- **文件级（阻断）**：读盘失败、解析失败、就地编辑失败归入 `ConfigError` 的 `Read` / `Write` / `Parse` / `Edit` 四个变体（`crates/qingjian-platform/src/error.rs:9,15,21,29`），带文件路径与底层来源；这类错误中止本次写入或加载，不让半成品落盘。
- **值域级（回退）**：取值越界、非法组合由访问器 clamp 或回缺省；单项非法不拖累其他项。非法条目（如自定义短语）跳过并留痕，不中断整份加载。
- **报告分层**：技术细节（路径、底层 error）进 tracing 日志；给用户的一句话在壳层组装（与 ADR-0003 的可见性分工一致）。设置程序展示的 `config_error` 只放可读摘要，原始来源随日志留存。

### 2.6 UI 侧消费约定

1. 设置程序（`apps/settings`、`apps/windows/settings`）只通过 `qingjian-platform` 的公开 API 读写配置：`Config::load`、`set_value`、`set_array`、`write_template_if_missing`；不直接持有 `toml_edit::DocumentMut` 写盘。
2. 页面读配置走 `Config` 的公开字段或访问器；不绕过配置模型直接解析 TOML 文本。新增设置项时，UI 控件与配置键的对应关系由离屏核对测试守住（现状 `apps/settings/src/offscreen/config_keys.rs`）。
3. 旧版设置壳（macOS preferences、Windows settings）作为兜底保留同一套公开 API；其私有读取逻辑不反向影响配置模型。
4. 配置导出/导入继续以 `Config` 可往返为前提：导入内容先解析成 `Config` 验证通过再合并（`transfer.rs:36-39`）。

## 3. 被否掉的备选方案

### 3.1 一次性把 TOML 重构成全新嵌套 schema、批量改键名

否掉。老样本从 v0.1.0 起就保存在 `tests/config_history/`，用户也能手改文件；一次性改键会让所有老配置读不出，违背「老配置必须原样可读」。现状的双读模式已经验证可行，延续它逐域过渡。

### 3.2 每个 UI 页/每个壳各读各的配置段，绕开统一 `Config`

否掉。多事实源会导致同一键在不同入口解读不一致，也丢掉 `transfer.rs` 的统一导入导出与 `config_history` 回归基线。配置模型必须仍是唯一事实源。

### 3.3 把配置文件格式换成 JSON 或数据库

否掉。现状的模板注释、键顺序保留、原子写、行内表转标准表（`mod.rs:691-713`）、导入导出都建立在 `toml_edit` 之上；换格式没有收益，却要重写全部读写路径与老样本基线。

### 3.4 在 `qingjian-core` 定义配置类型供平台复用

否掉。Core 平台无关（ADR-0001 D1），配置含文件路径、平台快捷键名、界面语言等平台概念；配置类型下沉到 core 会把平台概念带进领域层。

### 3.5 用 `#[serde(flatten)]` 把 GeneralConfig 直接摊平成一页避免拆子结构

否掉。flatten 不解决字段归属与维护问题，还会带来键冲突、序列化位置丢失和往返测试缺口；摊平只是把 64 字段的现状原样冻住。

## 4. 后果

### 4.1 正面后果

- 配置域边界明确，后续新增设置项知道该归哪组、由谁消费。
- 键名兼容规则成文：重组结构不再威胁老配置与手改文件。
- 读写入口收束到公开 API，设置程序与壳层不再各自解析 TOML。

### 4.2 迁移成本与风险

- **成本**：双读期内旧键 `Option` 字段与访问器合并逻辑会并存一段周期；每拆一个域都要补往返测试与老样本核对。结构拆分本身在 `refactor/split-others` 工作线，本文不估其工时。
- **风险 1：旧键长期残留。** 双读字段若不定期评估到期，会随结构拆分继续膨胀；需要在落地步骤里安排到期复审。
- **风险 2：豁免额度。** `GeneralConfig` 64 字段目前占质量门豁免（`allowlist.toml:49`）；拆分降回 40 以内后要跑 `--update` 清豁免，否则门禁只升不降。
- **风险 3：flatten 与嵌套表的往返差。** serde 对 flatten、行内表的处理与手写模板不完全对称，必须靠 `template_parses_to_defaults` 与离屏逐键测试守住。
- **风险 4：UI 直读字段耦合。** Windows 旧版设置直接读公开字段（`component.rs:123` 起），结构重组时这些字段的访问器必须保留兼容过渡。

## 5. 分阶段落地步骤

每步可独立合并、可回滚；步骤 3 起的代码改动属 `refactor/split-others` 工作线，本文只规定衔接口径。

1. **步骤 1：定稿规则与域边界（本 ADR，文档）。** 不改代码。回滚：撤回本文档。
2. **步骤 2：配置键与旧键台账补全（文档）。** 对照 `TEMPLATE`（`mod.rs:255-530`）与 `tests/config_history/*.toml`，逐键登记：现行键、旧键、所属域、取值范围、缺省值。回滚：撤回台账文档。
3. **步骤 3：按域拆子结构（split-others 线）。** 一次只拆一个域：新子结构 + 公开访问器；原 `GeneralConfig` 上的旧访问器保留为 `#[deprecated]` 转发到新结构，公开 API 不 break。每拆一个域跑配置往返测试与老样本加载。任一步出问题，回退该域的子结构与转发层即可。
4. **步骤 4：清洗访问器补齐。** 新子结构按 2.4-1 补值域 clamp/回退；缺项补测试。
5. **步骤 5：旧键到期复审。** 观察若干版本，确认配置样本与现场反馈不再出现旧键后，删除 `Option` 旧字段与合并分支；删之前先在台账标注到期版本。
6. **步骤 6：清质量门豁免。** 字段数降到阈值内后更新 `tools/quality/allowlist.toml`；本 ADR 不直接改该文件。

## 6. 验证方式

- **配置回归测试**：`cargo test -p qingjian-platform --offline`，重点：
  - `every_released_template_loads`：`tests/config_history/` 全部老样本加载成功；
  - `template_parses_to_defaults`：模板序列化回读与缺省一致；
  - `partial_file_keeps_other_defaults`：缺字段文件取缺省；
  - 旧键双读测试（`appearance_reads_legacy_theme_key`、`files_written_before_the_code_key_keep_their_wubi` 等，`config/tests.rs:121` 起、`general/tests.rs:149` 起）。
- **UI 逐键核对**：`apps/settings` 的离屏配置核对（`src/offscreen/config_keys.rs`）：每个可写键读回后键名不认或值被改写即报错。
- **编译与门禁**：`cargo check -p qingjian-platform -p qingjian-settings --offline`；`python3 tools/quality/check_code_quality.py` 不新增超标项。
- **手工抽查**：任取 `tests/config_history/` 下一份老 TOML，经 `Config::load` 读取后，对照 1.1 表确认旧键字段都有归并访问器。

## 7. 相关 crate 的关系

- `qingjian-platform`：配置模型唯一所有者，对外暴露 `Config` 及各子配置类型（见 `src/lib.rs` 的 re-export 清单）。
- `qingjian-predict`：自带 `PredictConfig`，嵌入 `Config.predict`；其密钥解析 `resolve_api_key`（`predict/config.rs:80-88`）是配置消费方，不改模型。
- `apps/settings`、`apps/windows/settings`：配置消费端，只走公开 API。
- `apps/android/core`：配置由 Kotlin 经 JNI 传入内存结构（如 `CloudSettings`），不落盘到这份 TOML；与本模型的衔接边界见 ADR-0003。
- `qingjian-render`：只接收外观/字体/主题相关的纯数据，不持有配置模型。

## 8. 本 ADR 不包含什么

- **不执行结构拆分**：`GeneralConfig` 64 字段、`config/mod.rs` 729 行、`theme/mod.rs` 791 行的按域拆子结构是 `refactor/split-others` 工作线的代码交付；本文只定规则、边界与兼容口径，不写具体拆法。
- 不改任何 `.rs` / `.toml`，不新增或废弃配置项，不改模板文案与 i18n 字符串。
- 不规定设置程序的页面划分、控件与标题（归 `docs/design/settings.md`）。
- 不定义云端密钥的存储格式与轮换策略（现状 `api_key` / `api_key_env`，`predict/config.rs:21-24`），不处理密钥掩码实现细节（`platform::logs::secrets`）。
- 不重新设计跨进程协议类型（`protocol::*`），`InputSettings` 的字段增减不在本文范围。
