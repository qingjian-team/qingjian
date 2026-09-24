# 青简 MCP Server

把青简的查词 / 释义能力暴露成 [MCP](https://modelcontextprotocol.io) 工具，供 Claude Desktop、支持 MCP 的编辑器等客户端调用。

## 工具

| 工具 | 说明 | 参数 |
| --- | --- | --- |
| `lookup` | 按拼音输入串查候选词（与输入法同一排序与释义） | `input` 必填；`limit`（1–50，缺省 10）；`fuzzy`（模糊音，逗号分隔，`all` 全开） |
| `gloss` | 查一个词的学习语言译文 | `word` 必填 |

两条都走 Core 的真实链路（`Engine::query` / `Engine::annotate`），结果与输入法与 `apps/cli` 一致。

## 启动

在仓库根（让数据文件自动探测命中 `data/generated/` 或 `assets/lexicon` / `assets/glossary`）：

```bash
cargo run -p qingjian-mcp --
```

数据文件选择与 `apps/cli` 同一套逻辑（`.qj` 打包优先 → TSV → 随包数据 → 样例兜底）。启动参数：

- `--dict <词库>` / `--glossary <释义表>`：覆盖数据文件
- `--language <en|ja|es|zh>`：学习语言（缺省 en，释义表的语言）
- `--english <英文词表>`：中英混输词表

## 接入 MCP 客户端

以 Claude Desktop（`claude_desktop_config.json`）为例：

```json
{
  "mcpServers": {
    "qingjian": {
      "command": "/absolute/path/to/qingjian-mcp",
      "args": []
    }
  }
}
```

## 实现说明

- 传输：stdio 上的 JSON-RPC 2.0，`Content-Length` 分帧（MCP 规范同款），见 `src/protocol.rs`
- 会话：`initialize` / `notifications/initialized` / `tools/list` / `tools/call` / `ping`，见 `src/server.rs`
- 日志走 stderr，不污染 stdout 的协议帧
- 集成测试 `tests/mcp_session.rs` 起真实进程走完整会话