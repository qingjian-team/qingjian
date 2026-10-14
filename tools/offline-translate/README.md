# 青简离线翻译扩展

给青简现有的「翻译选中文字」接入中英双向本地模型，不需要在线 API 账号。适用于 Windows 版青简 0.2.1；Python 后端本身不依赖 Windows。

## 使用

- 词语：输入拼音后，点击候选右侧的译词，或按 `Ctrl + 数字` 直接上屏译词。
- 句子：先上屏中文，选中原文，按 `Ctrl + Alt + T`；候选出现后按 `Enter`、`Space` 或 `1` 替换选区。`Esc` 保留原文。
- 英文选区同样按 `Ctrl + Alt + T`，译回中文。
- 青简现有选区读取一次最多 500 个 UTF-16 单元。更长内容请分段选择；密码框和部分不能读取选区的应用仍受原版限制。
- 翻译是机器翻译，可在上屏前检查人名、术语和语气。

接口地址固定为本机回环地址。设置页沿用原版的「云服务」名称，但离线配置只请求 `127.0.0.1`，运行时禁止向外部地址建立连接。
普通候选联想不会运行翻译模型，也不读取剪贴板或记录原文。模型与依赖仅在安装阶段下载。

## 安装与启动

1. 安装官方青简 0.2.1，并关闭后重新打开要打字的应用，让新 DLL 生效。
2. 使用带 pip 的 Python 运行 `bootstrap.py --root "D:\输入法\离线翻译"`。它准备独立 Python 运行时、两个模型和固定版本依赖。
3. 运行 `install_profile.py --root "D:\输入法\离线翻译"`。原配置备份到扩展目录 `backups/`，输入方案和学习记录不删除。
4. 在 PowerShell 执行 `./start.ps1 -Root "D:\输入法\离线翻译"`，隐藏启动本地服务。
5. 登录自启快捷方式指向扩展目录的 `runtime/pythonw.exe`，参数为 `-X utf8 -m qingjian_offline --models "<扩展目录>\models" --token-file "<扩展目录>\local-token.txt"`。

支持的服务请求是青简已有的 OpenAI 兼容协议，路径 `/v1/chat/completions`。只有用户消息里的 `text` 与 `target_language` 同时存在且没有额外上下文字段时，才进入模型。
配置为 `automatic = false`，`lookback = lookahead = slots = 0`；普通联想、问字、词典兜底返回空内容，不调用模型。
配置自动检查更新也关闭，以便断网使用。

## 检查

```powershell
python -m unittest discover -s tools/offline-translate/tests -v
```

启动后 `http://127.0.0.1:17863/health` 应返回 `ready: true` 与 `offline: true`。
运行服务不会调用下载器；用户请求和译文不写日志。

## 模型与来源

- [Argos 模型索引](https://github.com/argosopentech/argospm-index)：`translate-zh_en-1_9` 与 `translate-en_zh-1_9`。
- 模型为 OPUS-MT 派生，包内 README 记录作者 Jörg Tiedemann、Santhosh Thottingal 和 CC-BY 4.0 许可。
- [CTranslate2](https://github.com/OpenNMT/CTranslate2) CPU int8 推理和 [SentencePiece](https://github.com/google/sentencepiece) 分词。
- 独立 Python 3.12.10 嵌入式运行时来自 Python.org；其发布摘要在安装时校验。
- 模型包做 ZIP 完整性检查，下载来源、大小与 SHA256 保存于本机 `assets.json`。

原生库在 Windows 上读取中文路径不可靠，因此模型由 Python 读入内存后交给推理库；无需改动用户的 D 盘目录。
