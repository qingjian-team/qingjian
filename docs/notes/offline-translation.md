# 离线翻译扩展

`tools/offline-translate` 是独立本地推理后端，复用现有 `PredictionKind::Translate`、选区读取、候选评审与上屏流程。
不改 Core 的候选排序，不在 TSF 壳里调用翻译库。使用教程见同目录工具的 README 与用户文档「问字与翻译」。

部署配置将 `[predict]` 接到 `http://127.0.0.1:17863/v1`，关闭自动联想并把上下文长度设为零。
服务只对 `{text, target_language}` 运行模型，其他请求返回空结果；`/health` 仅报告就绪状态。
绑定回环地址，使用本机随机令牌验证调用，禁用请求日志，推理运行时的审计钩子拒绝外部 `socket.connect`。

模型是 Argos 打包的 OPUS-MT 中文→英语和英语→中文 1.9，加载 CTranslate2 CPU int8；不依赖 Stanza、PyTorch 或在线 API。
保留换行与空段落，超过 180 个输入 token 的段落分块翻译，避免推理库默认截断输入。服务最多接受 10000 字符；TSF 仍遵循原版 500 UTF-16 单元的选区限制。
Windows 中文路径由 Python 读取 `sentencepiece.model` 与模型字节，传给原生库的内存接口；输出清除 SentencePiece 空格标记。

验证覆盖请求过滤、令牌、错误返回、体积限制、换行、外连阻断、配置保留与可恢复备份。
本机真实双向模型测试成功。2026-10-14 用户在实际应用中完成「你好，今天下午我们一起去公园散步。」→「Hello, we'll go for a walk to the park this afternoon.」并确认译文上屏。
