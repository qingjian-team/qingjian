# ADR-0001 分层与依赖方向规则

- 状态：已接受（Accepted）
- 日期：2026-11-20
- 适用范围：workspace 全部 17 个内部 crate 与 `apps/` 下的壳程序、`tools/` 下的工具 crate
- 相关 ADR：[0002-config-model.md](0002-config-model.md)（配置模型归属平台适配层）、[0003-error-model.md](0003-error-model.md)（错误类型的跨层转换约定）

## 1. 背景

### 1.1 现状依赖图（按 `Cargo.toml` 实测，非估算）

workspace 成员清单见根 `Cargo.toml:3-22`。各 crate 的内部依赖（`[dependencies]` 段，不含 dev-dependencies）实测如下：

| crate | 内部依赖 | 证据 |
|---|---|---|
| qingjian-format | 无 | `crates/qingjian-format/Cargo.toml` |
| qingjian-code | 无 | `crates/qingjian-code/Cargo.toml` |
| bamboo-slip-i18n | 无 | `crates/bamboo-slip-i18n/Cargo.toml` |
| qingjian-dictionary | qingjian-format | `crates/qingjian-dictionary/Cargo.toml:13` |
| qingjian-translate | qingjian-core、qingjian-format | `crates/qingjian-translate/Cargo.toml:13-14` |
| qingjian-learning | qingjian-core、qingjian-dictionary、qingjian-translate | `crates/qingjian-learning/Cargo.toml:15-17` |
| qingjian-lm | qingjian-core、qingjian-format | `crates/qingjian-lm/Cargo.toml:13-14` |
| qingjian-neural | qingjian-core、qingjian-format | `crates/qingjian-neural/Cargo.toml:24-25` |
| qingjian-predict | qingjian-core | `crates/qingjian-predict/Cargo.toml:14` |
| qingjian-voice | qingjian-core | `crates/qingjian-voice/Cargo.toml:14` |
| qingjian-core | bamboo-slip-i18n、qingjian-code、qingjian-dictionary | `crates/qingjian-core/Cargo.toml:13,17-18` |
| bamboo-slip | 无内部依赖 | `crates/bamboo-slip/Cargo.toml` |
| qingjian-render | bamboo-slip | `crates/qingjian-render/Cargo.toml:10` |
| qingjian-platform | bamboo-slip-i18n、qingjian-core、qingjian-dictionary、qingjian-format、qingjian-predict | `crates/qingjian-platform/Cargo.toml:13-17` |
| qingjian-clipboard-sync | 无 | `crates/qingjian-clipboard-sync/Cargo.toml` |
| qingjian-clipboard | qingjian-clipboard-sync、qingjian-code | `crates/qingjian-clipboard/Cargo.toml:14-15` |
| qingjian-update | qingjian-platform | `crates/qingjian-update/Cargo.toml:13` |

壳与工具（`apps/`、`tools/`）依赖方向实测：

- `apps/settings/Cargo.toml:11-30,50`：依赖 bamboo-slip、render、platform、core、predict、update、dictionary、format、learning、translate、clipboard，是唯一跨层消费大部分 crate 的聚合端。
- `apps/windows/tsf/Cargo.toml:15-16,20`：依赖 core、platform，并以路径依赖 `../server`（`qingjian-windows-server`）。
- `apps/linux/ibus/Cargo.toml:16,33`：正式依赖只有 platform；core 在 dev-dependencies 里，仅供造测试数据（见该文件 31-33 行注释）。
- `apps/android/core/Cargo.toml:16-25,33`：装配 Engine 的壳 crate，直接依赖 core、dictionary、learning、lm、predict、render、translate、voice、clipboard-sync、bamboo-slip-i18n。
- `apps/linux/server`、`apps/windows/server`、`apps/macos`、`apps/cli` 为 Engine 装配端，聚合领域 crate 的错误类型（错误面见 ADR-0003）。

### 1.2 现有约束与问题

1. 架构上已有「Core 平台无关、平台层不写排序与词库逻辑」的约束（`docs/design/architecture.md:26-27`），但这条约束目前只写在文档里，没有一份把 17 个 crate 逐一定层、把允许/禁止边列成表的规则。
2. 依赖方向靠约定维持：新增一个内部 `use` 或 Cargo 依赖时，没有可机检的对照表说明该 crate 能否被对方依赖。
3. 存在两处需要登记的非典型边：`qingjian-clipboard → qingjian-code`（`crates/qingjian-clipboard/Cargo.toml:15`）、`apps/windows/tsf → apps/windows/server`（`apps/windows/tsf/Cargo.toml:20`）。它们方向上有理由，但应显式登记，而不是埋在 Cargo.toml 里。
4. 质量门目前只管文件行数、struct 字段数、impl 方法数、函数行数（`tools/quality/check_code_quality.py:204`：file_lines 800、struct_fields 40、impl_methods 40、fn_lines 150），不管 crate 间依赖方向。

## 2. 决策

### 2.1 四层划分

按「领域内核 → 渲染 → 平台适配 → 服务壳」分四层。编号只表示依赖允许的方向，不代表代码质量高低。

| 层 | 名称 | crate | 职责 |
|---|---|---|---|
| L0 | 基础叶子 | qingjian-format、qingjian-code、bamboo-slip-i18n | `.qj` 容器与零拷贝视图、形码规则、界面文案目录；不含业务流程，不依赖任何内部 crate |
| L1 | 领域层 | qingjian-core、qingjian-dictionary、qingjian-translate、qingjian-learning、qingjian-lm、qingjian-neural、qingjian-predict、qingjian-voice | 拼音解析、候选生成、排序、学习、词库、释义、本地推理、云客户端；平台无关，不读系统配置文件、不画窗口 |
| L2 | 渲染层 | bamboo-slip、qingjian-render | 场景树/布局/文字/动画引擎，以及「帧数据 + 主题 → 位图」的渲染器；不反向知道领域业务规则 |
| L3 | 平台适配层 | qingjian-platform、qingjian-clipboard-sync、qingjian-clipboard、qingjian-update | 配置文件读写、Server 与壳之间的协议类型、剪贴板及其设备间同步、检查更新；把领域能力包装成各壳可共用的形态 |
| L4 | 服务/壳层 | `apps/cli`、`apps/settings`、`apps/clipboard`、`apps/macos`、`apps/windows/{server,tsf,settings}`、`apps/linux/{server,ibus}`、`apps/android/core`、`tools/*` | 进程装配、系统框架接入（IMK/TSF/IBus/JNI）、设置界面、数据生成与评测工具 |

层内归类可随职责迁移：某个 crate 换层必须连同它的依赖边一起在本 ADR 的修订记录里说明。

### 2.2 允许的边

- **同层内部依赖允许**，但不得成环（cargo 对 workspace 无环有硬性保证，层表只审方向）。
- **上层可依赖下层**：L4 → L3/L2/L1/L0，L3 → L2/L1/L0，L2 → L1/L0，L1 → L0。
- **L3 可依赖 L1**：platform 已经依赖 core、dictionary、format、predict（`crates/qingjian-platform/Cargo.toml:14-17`），这是「平台层装配领域能力」的合法方向，不算反向边。
- **L4 可跨层聚合**：壳程序为装配 Engine 可以同时依赖 L1、L2、L3 的多个 crate，现状即如此（见 1.1）。
- dev-dependencies、build-dependencies 不受 L1/L2/L3 方向限制，但只能用于测试与构建脚本，不得出现在产出的链接依赖里；需要登记理由（参照 `apps/linux/ibus/Cargo.toml:31-33` 的现有做法）。

### 2.3 禁止的边（机检规则）

以下规则每条都能由脚本解析各 `Cargo.toml` 的内部依赖后断言，不依赖读源码：

- **D1（L1 不得依赖上层）**：L1 任一 crate 的 `[dependencies]` 不得出现 `qingjian-platform`、`qingjian-update`、`qingjian-clipboard`、`qingjian-clipboard-sync`、`qingjian-render`、`bamboo-slip`，也不得出现指向 `apps/` 的路径依赖。
- **D2（渲染层保持纯渲染）**：`bamboo-slip` 的 `[dependencies]` 不得出现任何 `qingjian-` 内部 crate；`qingjian-render` 的内部依赖只允许 `bamboo-slip`。
- **D3（下层不得感知壳）**：L0/L1/L2/L3 任一 crate 的 `[dependencies]` 不得出现指向 `apps/` 或 `tools/` 的路径依赖，也不得依赖 `apps/` 产物发布的 crate。
- **D4（反向边禁止）**：L3 不得被 L1/L2 依赖；L2 不得被 L1 依赖；L0 不得依赖任何内部 crate。新增依赖边若落在禁止方向上，编译期不拦，由依赖方向检查拦下。
- **D5（L4 内部边须登记）**：L4 各 crate 之间的路径依赖必须出现在第 2.5 节的例外台账里；未登记的壳→壳路径依赖视为违规。现有合法例外只有 `apps/windows/tsf → apps/windows/server`。

### 2.4 代码组织上的配套约束

- 领域层（L1）的公开类型不得包含平台概念（窗口、句柄、进程路径、系统快捷键名）；平台相关的类型放 L3 的 `protocol` 与配置模块。
- 渲染层（L2）只接收 L1/L3 给出的纯数据帧，不回调查询词库、不读配置文件；配置由壳组装后传入。
- 跨层共享的序列化类型归 L3（现状：`qingjian-platform::protocol`，见 `crates/qingjian-platform/src/lib.rs` 的 `pub mod protocol`），L1 不定义跨进程协议。

### 2.5 例外台账与到期机制

禁止边不允许「口头同意」。任何一条例外必须登记在下表，写清：编号、两端 crate、违反哪条规则、理由、期限（到哪个版本或哪项工作完成后到期）、责任人。到期后若仍需要，重新登记并说明延续理由；未到期的例外在依赖检查里放行，到期未续的按违规处理。

| 编号 | 边 | 违反规则 | 理由 | 期限 |
|---|---|---|---|---|
| EX-001 | `apps/windows/tsf` → `apps/windows/server` | D5 | TSF DLL 复用 Server 的协议与装配类型；DLL 不能带 Engine 依赖树，两个 package 分开（依据 `CLAUDE.md:35`） | 长期，随 Windows 双 package 结构存在；结构合并后复审 |
| EX-002 | `qingjian-clipboard` → `qingjian-code` | 2.2 备注 | 剪贴板条目解析复用形码规则；方向为 L3 → L1，本属合法，登记备查，不占例外额度 | 随 clipboard 模块迁移复审 |

新增例外走 ADR 修订（在本节追加一行），不允许只在代码注释里说明。

## 3. 被否掉的备选方案

### 3.1 按目录名机械分层（`crates/` 全算一层，`apps/` 全算一层）

否掉。现状 `crates/` 里既有零内部依赖的叶子（format、code、i18n），也有装配平台能力的 platform、update；若只分「库 / 程序」两层，D1/D2 这类「领域不许碰渲染与平台」的约束无法表达，等于把现有架构约束降级成目录约定。

### 3.2 引入一个新的共享内核 crate，把跨层公共类型都收进去

否掉。当前跨层共享类型集中在 `qingjian-platform::protocol` 与 core 的数据模型里，没有证据表明缺一个中间 crate；新设 crate 会再拉一层依赖、扩大公开 API 面，与「不一次性大重写、保持公开 API 兼容」的前提冲突。确有必要时再单独立 ADR。

### 3.3 用 cargo feature 或 workspace 角色代替层表

否掉。cargo 没有内置「层」概念，feature 解决的是编译期裁剪，不解决依赖方向审计；把层规则藏进 feature 配置会让方向约束不可见。本 ADR 选择显式层表 + 文本规则 + 脚本断言。

### 3.4 一次性把现有所有 crate 重新归类、移动目录

否掉。现状依赖图与本节层表基本吻合（1.1 表内没有已存在的反向边），不需要搬目录；归类调整随具体重构逐项进行，每次只动一两个 crate。

## 4. 后果

### 4.1 正面后果

- 新增内部依赖时，对照层表即可判断能否添加；评审有明确依据。
- L1 平台无关这条老约束变成可机检规则，Core 误引 platform/render 会被拦下。
- 非典型边（EX-001、EX-002）显式化，后续维护者不用反推 Cargo.toml 猜理由。

### 4.2 迁移成本与风险

- **成本**：需要一个脚本解析 workspace 各 `Cargo.toml`、对照层表出依赖违规清单；脚本实现不属本 ADR 交付（见第 7 节）。规则文本落地本身不改任何代码。
- **风险 1：误判合法边为违规。** L3 → L1、L4 跨层聚合是合法方向，脚本必须按 2.2 的方向表实现，不能简单禁止「高层名字出现在低层」。
- **风险 2：dev-dependencies 误报。** 测试用的内部依赖（如 ibus 的 core dev-dep）需在检查中单独排除并留痕。
- **风险 3：层归属争议。** predict、voice 是网络客户端，但当前归 L1；若后续把云服务配置与协议下沉到 L3，需要同步修订层表，而不是偷偷改依赖。

## 5. 分阶段落地步骤

每一步可独立合并、可回滚；任一步出问题，回退该步不影响其他步骤。

1. **步骤 1：定稿层表与规则文本（本 ADR）。** 不改代码，只把第 2 节的四层、允许边、禁止规则、台账写定。回滚：撤回本文档即可。
2. **步骤 2：补登记台账。** 核对 1.1 全表，确认没有遗漏的壳→壳路径依赖；把确认结果补进 2.5。此步只改文档。
3. **步骤 3：依赖方向检查脚本接入 CI（单独任务）。** 脚本读取 `cargo metadata` 或各 Cargo.toml，按 D1–D5 出报告；先以 warning 模式跑一个版本，只报告不阻断，对照现状确认零误报后再转阻断。脚本与 CI 配置不在本 ADR 范围内改动。
4. **步骤 4：存量例外复审。** 随下一次发布复审 EX-001、EX-002 是否仍成立；不成立的边在对应重构里拆除，台账行同步删除。

## 6. 验证方式

- **依赖图核对**：在仓库根执行 `& 'C:\Users\lbx13\.cargo\bin\cargo.exe' +1.96.0-x86_64-pc-windows-msvc metadata --no-deps --format-version 1`（离线环境加 `--offline`），解析各成员的内部 path/ workspace 依赖，与 1.1 表逐条对照。
- **编译回归**：`cargo check -p qingjian-core -p qingjian-platform --offline` 通过；Core 不引 platform/render 是 D1 的直接体现。
- **现有测试**：`cargo test -p qingjian-platform --offline` 通过（协议与配置回归基线，含 `tests/protocol/` 下的帧与变体测试）。
- **规模门禁不回退**：`python3 tools/quality/check_code_quality.py` 维持现状通过；本 ADR 不新增超标项，存量豁免见 `tools/quality/allowlist.toml`。
- **规则可机检性**：D1–D5 每条都只依赖 Cargo.toml 里的依赖名与路径，不要求解析 Rust 源码；脚本可用「crate 名 → 层」映射表 + 依赖边遍历实现。

## 7. 相关 crate 的关系说明

- **L1 内部**：core 是汇聚点；dictionary/translate/learning/lm/neural/predict/voice 各自面向单一能力，向上被 core 或壳装配。
- **L2 与 L1**：render 只依赖 bamboo-slip；候选数据模型在 core，渲染器不持有词库与排序依赖。
- **L3 与 L1**：platform 装配 core/dictionary/format/predict；update 依赖 platform 取 `UpdateConfig`、`UpdateChannel`（`crates/qingjian-update/src/checker/mod.rs:11,13`）；clipboard 经 clipboard-sync 做设备间同步。
- **L4 与下层**：server 类壳装配 L1 能力；settings 是跨层聚合端；tsf/ibus 这类前端壳只经 platform 协议通信，不直接扛 Engine（`apps/linux/ibus/Cargo.toml:13` 注释）。

## 8. 本 ADR 不包含什么

- 不规定 crate 内部模块怎么拆、文件怎么组织；那是各 crate 自己的事，大块拆分由独立工作线交付。
- 不改动任何 `Cargo.toml` 依赖、不新增或删除 crate、不移动目录。
- 不定义配置项的分组与键名（见 ADR-0002），不定义错误类型与转换（见 ADR-0003）。
- 不实现依赖方向检查脚本本身，也不修改 `tools/check_deps.py` 或现有质量门脚本；本文只给出规则与验收口径。
- 不重新讨论 `docs/design/architecture.md` 已有的产品级架构约束；本 ADR 是它在 crate 依赖层面的细化，冲突时以产品约束为准并回过来修订本文。
