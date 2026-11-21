# 新增的质量 job

`ci.yml` 负责“能不能编、测试过不过、依赖许不许”；这里补的是它没覆盖的四个维度：

- **code-quality**（`tools/ci/quality_gate.py`）：把 `unsafe`、`unwrap/expect`、
  `panic!/todo!/dbg!`、`#[allow]` 的总量钉在 `quality-baseline.toml` 上。只许降不许涨；
  确实合理的新增，改基线并在 PR 里说明理由。
- **docs-links**（`tools/ci/check_docs_links.py`）：仓库内 Markdown 的相对链接必须指向真实文件。
  外链不查（网络抖动会让 CI 假红）。
- **features**：`qingjian-predict` 的 `--all-features` 组合（native-tls + rustls 同时打开），
  防止“单开都能编、一起开就炸”。
- **release-tests**：`qingjian-core` 在 release profile 下跑一遍测试。`debug_assert!`、
  `#[cfg(debug_assertions)]` 两侧的代码在 release 下是另一份，只跑 debug 测试发现不了。

维护：

```sh
python3 tools/ci/quality_gate.py            # 本地跑门禁
python3 tools/ci/quality_gate.py --update   # 重构后收紧基线，跟代码一起提交
python3 tools/ci/check_docs_links.py
```
