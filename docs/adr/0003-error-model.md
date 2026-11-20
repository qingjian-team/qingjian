# ADR-0003 错误模型统一

- 状态：已接受（Accepted）
- 日期：2026-11-20
- 适用范围：workspace 各 crate 的错误类型、跨 crate 转换、IPC/FFI 边界降级，以及日志与用户提示的分工
- 相关 ADR：[0001-dependency-direction.md](0001-dependency-direction.md)（错误类型的依赖方向）、[0002-config-model.md](0002-config-model.md)（`ConfigError` 的文件级/值域级划分）

## 1. 背景

### 1.1 现状：各 crate 自带 thiserror 枚举（实测清单）

每个内部 crate 与壳程序各自定义错误枚举，全部基于 `thiserror`。实测文件与变体概览：

| crate / app | 错误类型 | 文件与行号 | 主要变体 |
|---|---|---|---|
| qingjian-platform | `ConfigError` | `crates/qingjian-platform/src/error.rs:7` | `Read`(9)、`Write`(15)、`Parse`(21)、`Edit`(29)；底层 `toml`/`toml_edit` 错误装箱（17-19 行注释说明体积原因） |
| qingjian-core | `ParseError` | `crates/qingjian-core/src/parser/error.rs:4` | `Empty`、`InvalidChar`、`NoSegmentation` |
| qingjian-dictionary | `DictionaryError` | `crates/qingjian-dictionary/src/error.rs:4` | `Io`、`Line`、`Format`、`Corrupt`、`InvalidCode`、`NoCodeEntries`、`CodeWords`、`InvalidWordRule`、`NoWordEntries` |
| qingjian-format | `FormatError` | `crates/qingjian-format/src/error.rs:5` | `Io`、`BadMagic`、`UnsupportedVersion`、`WrongKind`、`Truncated`、`MissingSection`、`Malformed`、`MetadataParse`、`MetadataSerialize`、`SectionTooLarge`、`UnknownCompression`、`TooLarge` |
| qingjian-learning | `LearningError` | `crates/qingjian-learning/src/error.rs:4` | `Io` |
| qingjian-lm | `LmError` | `crates/qingjian-lm/src/error.rs:6` | `Io`、`Format`、`Container`、`Corrupt` |
| qingjian-predict | `PredictError` | `crates/qingjian-predict/src/error.rs:6` | `MissingApiKey`、`Runtime`、`Api`、`Timeout`、`EmptyReply`、`BudgetExhausted`、`Encode`；`is_account_failure()` 24-33 行 |
| qingjian-translate | `GlossaryError` | `crates/qingjian-translate/src/error.rs:5` | `Io`、`Line`、`Container`、`Corrupt` |
| qingjian-update | `UpdateError` | `crates/qingjian-update/src/error.rs:6` | `Request`、`Runtime`、`TooLarge`、`MalformedSignature`、`UntrustedSignature`、`Parse`、`UnsupportedSchema`；5 行注释：一律只记日志，不打扰输入 |
| qingjian-voice | `VoiceError` | `crates/qingjian-voice/src/error.rs:6` | `MissingApiKey`、`Connect`、`Rejected`、`Server`、`Closed`、`Timeout`、`Protocol`、`Socket`；`describe()` 34-52 行给用户一句话 |
| qingjian-neural | `NeuralError` | `crates/qingjian-neural/src/error.rs:8` | `Io`、`Json`、`NotFound`、`Format`、`Candle`、`Onnx`、`Corrupt` |
| qingjian-clipboard | `ClipboardError` | `crates/qingjian-clipboard/src/error.rs:4` | `BadConfig(String)`、`Database`、`Directory` |
| qingjian-clipboard-sync | `SyncError` | `crates/qingjian-clipboard-sync/src/error.rs:7` | `Io`、`Handshake`、`UnknownPeer`、`NotPaired`、`Protocol(String)`、`Storage(String)`；`From<snow::Error>` 31-34、`From<serde_json::Error>` 36-38 |
| bamboo-slip | `RenderError` | `crates/bamboo-slip/src/error.rs:5` | `NoUiFont`、`InvalidSize`、`Layout` |
| bamboo-slip host | `HostError` | `crates/bamboo-slip/src/host/error.rs:4` | `EventLoop`、`Window`、`Surface`、`Render` |
| qingjian-render | `PreviewError` / `ShotError` | `crates/qingjian-render/src/preview/error.rs:7,17` | `Theme`、`Fonts`、`OutDir`；`Render`、`Save` |
| qingjian-render theme | `ThemeError` | `crates/qingjian-render/src/theme/error.rs:5` | `Io`、`Json`、`UnknownBase`、`TooDeep` |
| platform 协议 | `CodecError` | `crates/qingjian-platform/src/protocol/codec.rs:14` | `Io`、`Json`、`TooLarge` |
| apps/android/core | `AndroidError` | `apps/android/core/src/error.rs:7` | `Dictionary`、`Render` |
| apps/cli | `CliError` / `ColdError` | `apps/cli/src/error.rs:12`、`apps/cli/src/cold/error.rs:5` | 透明聚合各领域错误 + CLI 自身变体 |
| apps/linux/ibus | `IbusError` | `apps/linux/ibus/src/error.rs:7` | `Connect`、`UntrustedPeer`、`Codec`、`Closed`、`Encode`、`Unexpected(String)` |
| apps/linux/server | `ServerError` | `apps/linux/server/src/error.rs:8` | `Dictionary`、`Glossary`、`LanguageModel` |
| apps/macos | `HostError` | `apps/macos/src/error.rs:8` | `NoResources`、`MissingResource`、`Dictionary`、`Glossary`、`Learning`、`Config`、`LanguageModel`、`Emoji` |
| apps/windows/server | `ServerError` | `apps/windows/server/src/error.rs:8` | `Dictionary`、`Glossary`、`LanguageModel` |
| apps/windows/tsf | `ClientError` | `apps/windows/tsf/src/error.rs:6` | `Codec`、`Closed`、`Unexpected(&'static str)` |

### 1.2 粒度不一与字符串化的现状

1. **分类缺失**：现有枚举按「资源类型/外部库」分组，没有统一说明哪些错误可恢复、哪些需要告诉用户。可恢复性只能从装配代码的处理方式反推：例如可选数据加载失败记 warn 后继续（`apps/android/core/src/assembly.rs:38,45,59,109,160`；`apps/linux/server/src/assembly/mod.rs:44`、`assembly/cloud.rs:21-31`），主词库/渲染失败则经 `AndroidError`、`ServerError` 中止装配。
2. **边界处字符串化（丢弃来源）**：实测把底层错误 `.to_string()` 后装进字符串变体的点：
   - `crates/qingjian-clipboard/src/store/sync.rs:111`：`SyncError::Storage(error.to_string())`；
   - `crates/qingjian-clipboard-sync/src/identity.rs:24`、`peers/book.rs:24,26,96`：`SyncError::Storage(error.to_string())`；
   - `crates/qingjian-clipboard/src/settings/mod.rs:117`：`ClipboardError::BadConfig(path.display().to_string())`；
   - `crates/qingjian-voice/src/volcengine/stream.rs:69`、`crates/qingjian-voice/src/tls.rs:18,22,26,28,30,46`：`VoiceError::Connect(error.to_string())`；
   - `apps/linux/ibus/src/client/connection.rs:95`：`IbusError::Unexpected(reply.to_string())`；`apps/linux/ibus/src/client/reply.rs:72`：`IbusError::Unexpected(value.to_string())`。
3. **用户提示与日志混用**：只有 `VoiceError::describe()`（`voice/src/error.rs:34-52`）和安卓云测试的本地 `describe()`（`apps/android/core/src/cloud/mod.rs:25-35`）把错误翻成中文用户话；其余 `#[error]` 文案直接进日志，设置程序在重读失败时把 `ConfigError` 字符串化展示（`apps/settings/src/settings/config.rs:40`）。
4. **边界降级已有雏形**：
   - Windows TSF：按键请求失败时记日志、断开当前会话、结束组句、按键透传，下次按键重连（`apps/windows/tsf/src/com/service/key_sink.rs:379-387`）。
   - Linux IBus：`serve()` 失败记 `error!` 后退出 1（`apps/linux/ibus/src/main.rs:46-48`）；自检失败同样退出（61-63）。
   - Android JNI：测试连接回 `{ok, message}` JSON（`apps/android/core/src/jni/cloud.rs:14-20,112-132`）；`catch_unwind` 兜底 panic 成失败结果（119-123）；取云联想失败记日志返回 null（173-176）。

## 2. 决策

### 2.1 保留每 crate 自有枚举，不做全局大 Error

- 继续由各 crate 定义自己的 thiserror 枚举；壳程序（`apps/*`）用透明聚合枚举包裹领域错误（现状 `CliError`、`ServerError`、`HostError`、`AndroidError`、`ClientError`、`IbusError` 的模式）。
- 不引入跨 crate 的全局错误层级，不把所有变体塞进一个枚举。理由：现状来源链（`#[source]`、`#[from]`）已经可用，全局枚举会把领域 crate 与壳层重新耦合，属于一次性大改，与兼容前提冲突。

### 2.2 两个正交分类维度

每个错误变体必须能在下面两个维度上归类；分类用于决定降级方式与展示方式，不强制在枚举上新增标注字段。

**可恢复性**

- **可恢复（Recoverable）**：单项功能缺失或失败，输入法主体继续工作。典型：可选词库/释义表/语言模型/云服务/语音/剪贴板加载或调用失败；回退到本地候选、跳过该功能或等待下次重试。处理方式：记 warn 或 info，不弹阻断性错误。
- **不可恢复（Fatal）**：当前装配或会话无法继续。典型：主词库读不出、配置文件解析失败且无法按缺省启动、协议损坏导致会话状态不可信、窗口/字体根本建不起来。处理方式：记 error，中止装配或断开会话，走边界降级（2.5）。

**可见性**

- **用户可见（UserVisible）**：需要给用户一句可理解的话。典型：缺密钥、密钥无效/欠费（401-403）、网络连不上、服务拒绝、配置笔误导致某项用不了。
- **内部（Internal）**：只进日志，不打扰用户。典型：布局/缓存/内部状态错误、可重试的瞬时失败、调试细节。

一个错误可以是「可恢复 + 用户可见」（如云联想连不上，提示一句但继续打字），也可以是「不可恢复 + 内部」（如会话协议损坏，断开重连即可，用户无感知）。

### 2.3 跨 crate 转换约定

1. **保留来源链**：领域错误进入壳聚合枚举时，优先 `#[from]` 透明包裹；需要新变体时带 `#[source]` 持有底层错误，不把来源降级成字符串。
2. **禁止内部边界字符串化**：内部 crate（L0–L3，见 ADR-0001）之间不得用 `map_err(|e| e.to_string())` 或 `String` 变体丢弃底层错误；1.2-2 列出的存量点登记为待改项，随触及该代码时消除，改法是补带 `#[source]` 的变体或 `From` 实现。
3. **不用 `String` 做跨 crate 错误类型**：内部公开函数签名不得返回 `Result<T, String>`；`Result<_, String>` 只允许出现在 FFI/协议出口的适配层（安卓 JNI 回传、IPC 消息体），且出口处已经完成分类与文案组装。
4. **不引入 anyhow、不使用 `Box<dyn Error>` 跨 crate 返回**：保持 thiserror 枚举的类型可推断性；壳层聚合枚举是唯一的错误汇合点。
5. **转换方向单向**：领域 crate 不依赖壳的错误类型（同 ADR-0001 D1）；壳可以依赖领域错误，反向不允许。

### 2.4 日志与用户提示的分工

1. **`#[error]` 文案面向日志**：可以含路径、状态码、内部标识；用技术表述，不承诺给终端用户阅读。
2. **用户一句话在边界层组装**：需要用户可见时，由壳/协议边界提供 `describe()` 类方法（沿用 `VoiceError::describe` 的形态），输出本地化、简短、可行动的提示；领域 crate 不直接产出面向用户的 i18n 文案。
3. **分级对应可恢复性**：可恢复单项失败记 warn（瞬时/可重试可 info/debug）；不可恢复记 error；正常回退路径不刷错误日志。
4. **敏感信息不进错误与日志**：密钥、令牌、用户输入原文不进错误文案；沿用平台日志的密钥掩码登记机制（`Config::load` 中的 `secrets::register`，`config/mod.rs:557-560`）。
5. **设置程序展示**：配置类错误在 UI 上显示可读摘要，底层来源随日志留存；不把技术枚举名或路径直接抛给用户。

### 2.5 IPC / FFI 边界降级

#### 2.5.1 IPC 边界（Windows TSF ↔ Server、Linux IBus ↔ Server）

- 协议层错误统一归 `CodecError`（编解码/帧过大）、`Closed`（对端关闭）、`Unexpected`（消息不匹配）；对端不收到内部错误字符串，只收到类别与必要的会话动作。
- 会话级失败：断开当前会话、结束组句、按键透传给应用，下次交互重连（现状 TSF 行为，`key_sink.rs:379-387`）；不因为一次请求失败杀掉输入法进程。
- 装配级失败：前端连不上 Server 时按框架要求退出或保持空闲，错误进日志；不向应用进程注入半成品候选。
- 新增协议消息的错误只携带枚举类别，底层 `io`/`json` 来源留在本侧日志。

#### 2.5.2 FFI / JNI 边界（Android）

- 跨边界只传结构化结果：成功回数据或 `{ok: true, message}`，失败回 `{ok: false, message}` 或布尔 + null（现状 `jni/cloud.rs:14-20,112-132`）；不把 Rust 错误枚举或 `Box<dyn Error>` 暴露给 Kotlin。
- 出口适配层做最后一道兜底：`catch_unwind` 把 panic 转成失败结果（现状 `cloud.rs:119-123`）；JNI 函数不抛异常给 Kotlin，失败回退默认行为（空串、null、false）。
- 错误细节在 Rust 侧记日志；传给 Kotlin 的 `message` 必须是用户可见文案，不含路径与内部标识。

#### 2.5.3 配置边界

- 文件级 `ConfigError`（读不出、解析失败、就地编辑失败）按 ADR-0002 阻断本次写入/加载；值域级错误回退缺省，不升级成用户弹窗。

## 3. 被否掉的备选方案

### 3.1 定义全局统一 `QingjianError` 枚举 + 数字错误码

否掉。会把 17 个 crate 的错误面焊在一个类型上，领域改动要动全局枚举；现有 `#[source]` 来源链与壳层透明聚合已经够用，错误码表还会成为长期维护负担。确有跨进程错误分类需求时，只在协议层定义小类别枚举，不反向要求领域 crate 实现。

### 3.2 内部一律用 `String` 当错误类型

否掉。字符串化丢掉来源链、无法做 2.2 的分类与降级，也无法在编译期区分错误类型；现状 1.2-2 的字符串化点正是要收敛的对象，而不是推广方向。

### 3.3 引入 anyhow 作为边界错误类型

否掉。anyhow 抹除类型，壳层无法按变体决定降级路径；与现有 thiserror 体系双轨并存会增加转换成本。边界需要动态汇合时，用壳层自己的聚合枚举。

### 3.4 每个领域错误变体都直接带用户文案

否掉。领域 crate 会被 i18n 文案绑死，同一失败在不同壳（桌面/安卓/CLI）需要不同表述；现状 `VoiceError::describe` 放在边界侧的分工更合理，应推广而不是下沉。

### 3.5 在枚举上强制派生可恢复性/可见性标注

否掉。初期分类主要是语义约定，形式标注会给每个变体加样板；先靠评审与清单维持，若后续形式检查不足再单独立 ADR 引入标注属性。

## 4. 后果

### 4.1 正面后果

- 新增错误时有明确归类：能不能继续打字、要不要提示用户、日志记哪一级，对照 2.2 即可判断。
- 边界降级口径统一：IPC 断会话重连、JNI 回结构化结果，不再各壳各写一套。
- 来源链逐步恢复：字符串化存量点消除后，排障能看到底层原因。

### 4.2 迁移成本与风险

- **成本**：1.2-2 列出的字符串化点散在 voice、clipboard、clipboard-sync、ibus 四处，需要逐个补变体或 `From`；每个点配一条回归测试。盘点与消除不属本 ADR 的一次性交付。
- **风险 1：形式检查覆盖不到语义分类。** E1–E4 能查 `String` 返回、来源丢弃、依赖方向、文案归属；「可恢复/不可恢复」判断仍需评审，误判会导致该降级的被当成致命错误。
- **风险 2：describe 文案漂移。** 用户提示分散在各壳边界，同一类失败可能出现多种说法；需要在边界侧集中文案来源，避免每次新增壳重新发明。
- **风险 3：兼容期双轨。** 旧的字符串变体还在，新代码可能继续往字符串里塞；需要准入规则先立住，存量再慢慢清。

## 5. 分阶段落地步骤

每步可独立合并、可回滚；步骤 3 起涉及代码改动，按最小批次提交。

1. **步骤 1：定稿分类与约定（本 ADR，文档）。** 不改代码。回滚：撤回本文档。
2. **步骤 2：存量错误清单盘点。** 把第 1.1 节的枚举、1.2-2 的字符串化点、各边界降级现状登记进仓库文档；每个点标注归类（可恢复性/可见性）与待改方式。回滚：撤回清单文档。
3. **步骤 3：新错误准入规则落地。** 评审按 E1–E4 把关；能形式化的检查先以 warning 模式跑，对照现状确认零误报后再转阻断（脚本与 CI 配置不在本 ADR 改动范围）。
4. **步骤 4：消除存量字符串化。** 按 voice → clipboard-sync → clipboard → ibus 的顺序，把 `to_string()` 丢弃来源的点改成带 `#[source]` 的变体或 `From` 实现；每改一处跑对应 crate 测试。任一处出问题，回退该变体改动。
5. **步骤 5：边界 describe 收口。** 把需要用户提示的路径（predict 账户失败、网络拒绝、配置笔误等）按 `VoiceError::describe` 形态在壳/协议层补一句话；领域枚举不加用户文案。
6. **步骤 6：分类标注复审。** 积累一段时间后评估是否需要在错误枚举上显式标注可恢复性/可见性；需要时另立 ADR，本文不预设属性设计。

## 6. 验证方式

- **领域测试**：`cargo test -p qingjian-predict -p qingjian-voice -p qingjian-dictionary -p qingjian-format -p qingjian-neural --offline`；壳层聚合测试 `cargo test -p qingjian-cli --offline`。
- **形式检查（规则对应 E1–E4）**：
  - E1：内部 crate 公开签名不出现 `Result<_, String>`；错误类型均为 thiserror 枚举。
  - E2：内部 crate 不出现 `map_err(|e| e.to_string())` 或等价的来源丢弃；除步骤 2 清单登记的存量点外零命中。
  - E3：错误枚举所在 crate 不依赖 `apps/*`（与 ADR-0001 D1 一致，由依赖方向检查覆盖）。
  - E4：`describe` 类用户文案方法只出现在壳/协议边界 crate；领域 crate 的 `#[error]` 文案不含面向用户的本地化句子。
- **边界手工验证**：
  - 断开 Server 后敲键：TSF 按键透传、会话断开并可重连（`key_sink.rs:379-387` 路径）；IBus 前端记日志退出，不注入半成品候选。
  - 安卓断网点「测试连接」：回 `{ok: false, message}`，不向 Kotlin 抛异常；panic 被 `catch_unwind` 兜成失败结果。
- **门禁**：`python3 tools/quality/check_code_quality.py` 不新增超标项；本 ADR 不改动现有错误变体的公开命名。

## 7. 相关 crate 的关系

- **L1 领域 crate**：定义自身错误枚举，只经 `#[from]`/`From` 向上暴露；不依赖壳错误类型；`PredictError::is_account_failure` 这类判定方法留在领域层供边界组装文案使用。
- **L3 平台适配层**：`ConfigError`、`CodecError` 是跨进程与配置边界的共享错误；`ConfigError` 的细分见 ADR-0002。
- **L4 壳程序**：聚合领域错误为本地枚举（`CliError`、`ServerError`、`HostError`、`AndroidError`、`ClientError`、`IbusError`），在此完成降级决策、日志分级与用户文案组装。
- **渲染层**：`RenderError`、`ThemeError`、`PreviewError`/`ShotError` 表达素材与绘制失败；建不出字体/窗口属不可恢复，单张预览失败可恢复。

## 8. 本 ADR 不包含什么

- 不改任何现有错误类型、变体名与 `#[error]` 文案；不重命名、不删除现有 `describe` 方法。
- 不设计全局错误码表、HTTP 状态码映射表，也不建设 i18n 用户文案库。
- 不调整日志框架、订阅器与输出配置（沿用 tracing 现状）；不定义日志脱敏的完整规则，只延续现有密钥掩码机制。
- 不覆盖 panic 捕获策略的全面设计；仅延续 JNI 出口 `catch_unwind` 这一道边界兜底。
- 不规定各壳的 UI 弹窗样式与提示措辞；那是壳层与产品文案的事。
