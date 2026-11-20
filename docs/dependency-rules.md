# 架构依赖规则

这一页是 workspace 内部依赖方向的机器校验规则：每条规则为什么存在、脚本怎么验、违规怎么豁免，以及新增 crate 时的落地清单。
校验脚本是 [../tools/check_deps.py](../tools/check_deps.py)，豁免清单是 [../tools/deps-allowlist.toml](../tools/deps-allowlist.toml)，CI 在 `architecture` job 里跑同一条命令。

## 数据源与口径

- 脚本只消费 `cargo metadata --format-version 1` 的结构化输出，**不解析任何 Cargo.toml 文本**；依赖种类（normal / dev / build）、target 条件、features 全部取自 metadata。
- 内部依赖按 manifest 路径归类到三层：`crates/`（库）、`apps/`（平台前端壳）、`tools/`（一次性命令行工具）；metadata 里的外部传递依赖不参与方向判断。
- 正常依赖图（normal + build）与 dev 依赖图分别成环检查，不混在一起。
- 本地离线跑：`python3 tools/check_deps.py --offline`；联网环境（含 CI）去掉 `--offline`。脚本不把 `--offline` 写死。
- 输出每条违规一行机器可读记录：

  ```
  VIOLATION rule=<规则名> src=<来源 crate> dst=<被依赖 crate> kind=<normal|dev|build> target=<cfg 条件或 -> evidence=<manifest 路径与依赖证据>
  ```

  有未登记违规时退出码为 1，否则为 0。

## 分层模型

| 层 | 位置 | 职责 |
|---|---|---|
| 引擎内核 | `crates/qingjian-core` | 拼音解析、候选生成、整句转换、排序、学习接口；平台无关 |
| 兄弟库 | `crates/` 其余 crate | 词典、翻译、学习、格式、预测、语音、语言模型、神经打分、渲染、UI 引擎、更新、剪贴板、代码识别等 |
| 渲染层 | `crates/bamboo-slip`、`crates/qingjian-render` | 自绘 UI 引擎与候选窗渲染器，只出位图与布局，不碰业务服务 |
| 服务层 | `crates/qingjian-predict`、`qingjian-translate`、`qingjian-neural`、`qingjian-lm`、`qingjian-voice`、`qingjian-update` | 联想、翻译、神经打分、语言模型、语音识别、更新检查等业务能力 |
| 前端壳 | `apps/` | 各平台输入法壳与设置程序，只做事件翻译与贴图 |
| 工具 | `tools/` | 词典转换、词表生成、签名等离线命令 |

## 规则清单

下表「当前例外」是基线 `7de460d5` 上实测冻结的数字；豁免条目见 `tools/deps-allowlist.toml`。

### 1. `core-forbidden`：core 不得依赖平台 / 渲染 / 服务 / 剪贴板 / code 层

- **为什么**：`qingjian-core` 是平台无关的引擎内核，被所有前端壳直接依赖。它一旦反向依赖平台层、渲染层或某个业务服务，内核就被绑死在那条产品线上，换前端要改 core，平台解耦这条底线就破了。
- **怎么校验**：`qingjian-core` 的每条内部依赖边，目标不在禁止清单内。禁止清单：`qingjian-platform`、`qingjian-render`、`bamboo-slip`、`qingjian-predict`、`qingjian-neural`、`qingjian-lm`、`qingjian-voice`、`qingjian-update`、`qingjian-clipboard`、`qingjian-clipboard-sync`、`qingjian-code`。
- **怎么豁免**：在 `[allow]` 登记 `core-forbidden|qingjian-core|<目标>`，写明理由与到期日；core 需要的通用能力应下沉为更底层的工具 crate，或改由下游直接依赖，而不是让 core 长期转发。
- **当前例外**：1 条（`qingjian-core → qingjian-code`，normal，再导出公共 code 模块，待下沉或改为下游直连）。

### 2. `render-to-service`：渲染层不得依赖业务服务层

- **为什么**：渲染层只负责场景树、布局、文字与位图，不做联想、翻译、打分、取模型、联网这类业务决策。渲染器被服务层依赖（单向向下）才是干净方向；反过来会让 UI 引擎绑定具体业务，预览、测试与复用都被拖住。
- **怎么校验**：`bamboo-slip`、`qingjian-render` 不得依赖 `qingjian-predict`、`qingjian-translate`、`qingjian-neural`、`qingjian-lm`、`qingjian-voice`、`qingjian-update`。
- **怎么豁免**：登记 `render-to-service|<来源>|<目标>`；属于渲染层确实需要的通用数据结构，应上移到 core 或独立类型 crate，而不是渲染层去拉服务层。
- **当前例外**：0 条。

### 3. `crate-to-shell`：crates 不得反向依赖 apps / tools

- **为什么**：库 crate 是被前端壳和工具调用的一方。crates 依赖 `apps/` 或 `tools/` 会形成库→壳的反向边，库脱离壳就编不过，复用与测试都被污染。
- **怎么校验**：`crates/` 成员的内部依赖目标不得落在 `apps/` 或 `tools/`。
- **怎么豁免**：登记 `crate-to-shell|<来源>|<目标>`；壳里可复用的逻辑应下移到 crate。
- **当前例外**：0 条。

### 4. `shell-to-shell`：apps / tools 只能依赖 crates 与外部依赖，不得互相依赖

- **为什么**：`apps/` 是各平台前端，`tools/` 是离线工具。壳与壳、壳与工具之间直接拉依赖，会把平台专有逻辑缠成一团，换平台、跑单测都要带上对方。壳需要的共享能力一律走 `crates/`。
- **怎么校验**：`apps/`、`tools/` 成员的内部依赖目标只能是 `crates/`；目标落在另一个 `apps/` 或 `tools/` 即违规。
- **怎么豁免**：登记 `shell-to-shell|<来源>|<目标>`，并注明是否仅 dev-dependency；测试场景需要对端宿主时，应抽独立 test-support crate 或改用 IPC mock。
- **当前例外**：1 条（`qingjian-windows-tsf → qingjian-windows-server`，仅 dev，端到端协议测试链接 server 作宿主，不进发布产物）。

### 5. `cycle-normal`：正常依赖图不得成环

- **为什么**：正常（含 build）依赖成环意味着编译顺序无法确定，cargo 只能报循环依赖；也通常对应职责切分不干净，两个 crate 互相拿对方的类型。
- **怎么校验**：在 normal + build 内部边上做 DFS 环检测；出现环即失败，环上节点写进 evidence。
- **怎么豁免**：正常环不设豁免通道——拆环（下移共享类型、合并 crate、或引入中间接口）后环自然消失。
- **当前例外**：0 个环。

### 6. `cycle-dev`：dev 依赖图成环必须登记

- **为什么**：dev 依赖只进测试 / examples / 宏展开，不进发布产物，偶尔会出现测试侧的小环（端到端测试互链）。这类环单独允许，但必须登记在案，避免测试依赖悄悄长成正式耦合。
- **怎么校验**：在 dev 内部边上单独做环检测；未登记的 dev 环失败，登记过的放行。
- **怎么豁免**：在 `[dev-cycles]` 按 `<a>|<b>|<c>` 登记环路径，填 reason 与 expires；测试支撑抽出后删条目。
- **当前例外**：0 个环。

### 7. feature 卫生提示（非阻断）

- **为什么**：同一个依赖被多个 crate 以不同 feature 组合引入，长期会让编译面分叉、漏编某些 cfg 分支。这条只提示不失败，给收敛留窗口。
- **怎么校验**：按依赖名分组，比较 `default-features` 与显式 features 集合；组合多于一种就打 `HINT` 行。
- **怎么豁免**：不需要登记；发现可收敛的组合时，优先复用根 `[workspace.dependencies]` 的统一声明。
- **当前提示**：8 组依赖存在多组合（含 `arboard`、`async-openai`、`reqwest`、`tokio`、`windows` 及部分内部 crate 的平台特性）。

## 豁免机制（与代码规模质量门同风格）

豁免文件：`tools/deps-allowlist.toml`。

- **登记即冻结现状**：只有清单里精确匹配 `规则|src|dst` 的边放行；未登记的新违规直接失败。
- **条目字段**：
  - `reason`：为什么暂时不能改、计划往哪改；不能为空。
  - `expires`：到期日期 `YYYY-MM-DD`，或 `permanent`（长期保留必须在 reason 里说明）；过期条目按失败处理，需续期或移除。
  - `kinds`：登记时覆盖的依赖种类（`normal` / `dev` / `build`）。同一条边冒出清单外的新 kind，视为违规变大，失败。
- **只减不增**：重构消除违规后删掉对应条目即可；脚本不自动删除已登记项。登记过但已不对应任何违规的条目，运行时会打 `NOTE` 提示清理。
- **不为清零放宽规则**：基线违规按实测冻结；不许通过改规则定义、合并规则或把禁止清单改空来让违规消失。

## 新增 crate 落地检查清单

加一个 workspace crate 前，按顺序过一遍：

1. **选对层**：平台无关的引擎 / 数据能力进 `crates/`；平台前端进 `apps/`；离线命令行进 `tools/`。不确定放哪时先看 [design/architecture.md](design/architecture.md) 的分层说明。
2. **登记成员**：把路径加进根 `Cargo.toml` 的 `members`（根 manifest 由仓库维护者统一改；脚本只读它做路径归类）。
3. **方向自查**：
   - 不依赖本层禁止清单里的 crate（core 见规则 1，渲染层见规则 2）；
   - `crates/` 不引用任何 `apps/`、`tools/`；
   - `apps/`、`tools/` 只依赖 `crates/` 与外部 crate，不依赖别的壳或工具；
   - 正常依赖与 dev 依赖都不要成环。
4. **依赖声明**：外部版本与 features 优先复用根 `[workspace.dependencies]`；新 feature 组合留意脚本的 `HINT` 提示，能合并就合并。
5. **本地验证**：在仓库根跑 `python3 tools/check_deps.py --offline`（联网环境去掉 `--offline`），退出码 0 再提交；有存量违规需要豁免时，先在 `tools/deps-allowlist.toml` 写清 `reason` 与 `expires`。
6. **CI 复核**：PR 会触发 `architecture` job，与本地同一条命令；job 失败按 `VIOLATION` 行里的 `rule` 与 `evidence` 定位。

## 基线快照（`7de460d5`，2026-11-20）

- workspace 内部依赖边：109 条（normal 104，dev 5）；正常图 0 环，dev 图 0 环。
- 阻断违规：2 条，全部已登记放行；未登记违规 0 条。
- feature 组合提示：8 组（非阻断）。
