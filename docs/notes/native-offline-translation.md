# 原生离线选区翻译

Windows 选区翻译在现有 Server 进程中执行本地 ONNX 推理；不依赖 Python、HTTP 服务或 API 密钥。首次使用在「设置 → 实验功能」明确下载中英双向模型，下载后断网可用。

## 数据流与取消

`选区快捷键 → TSF 读取 → Router → Engine::request_translation → NativePredictor → CPU ONNX → Engine::poll_prediction → 译文候选 → 用户确认`。

Core 独立的 `translation_predictor` 只接显式选区请求。普通候选联想、停键翻译和释义仍走原来的提供方。配置选择本地翻译后，模型或目标语言不支持时返回失败，不自动转在线服务。私密输入不提交；取消和失焦使请求代次过期，迟到结果无法写入新组句。

只维护一个推理线程和一个待发请求槽，后来的请求覆盖尚未执行的请求。解码逐步检查取消与60秒任务期限；ONNX单次运行不能强制中断，但在后台执行，不阻塞输入线程。关闭时取消并唤醒工作线程。

## 选区兼容与确认

标准 TSF 选区保存原范围、原文与焦点，回填前重新校验。应用提供空选区时，Scintilla 控件使用本线程只读消息；其他目标由独立 MTA 工作者读取 UIA TextPattern。只接受同一前台进程、有键盘焦点、非密码框的单个选区，最多500个UTF-16单元。

备用读取没有可靠的 TSF 文档锚点，因此只在用户确认后复制译文，再提示手动粘贴。取消和读取失败不修改剪贴板。读取只有一个工作者，收结果最多等待5秒；未响应的提供方可能继续占用该工作者，后续请求明确提示忙碌。没有模拟按键、自动复制原文或自动改变焦点。

TSF 没有上下文、拒绝编辑会话、未回调以及UIA未提供选区的应用仍可能不支持翻译，不能把备用通道视为所有应用都可读。协议10的配套DLL和Server才支持完整等待、备用读取和复制提示；升级后应重开宿主应用。

## 模型来源与安装

- 原始模型：[Helsinki-NLP/opus-mt-zh-en](https://huggingface.co/Helsinki-NLP/opus-mt-zh-en)、[Helsinki-NLP/opus-mt-en-zh](https://huggingface.co/Helsinki-NLP/opus-mt-en-zh)，作者为赫尔辛基大学 Language Technology Research Group，中译英采用CC-BY-4.0，英译中采用Apache-2.0；以各原始模型卡为准。
- ONNX导出：[Xenova/opus-mt-zh-en](https://huggingface.co/Xenova/opus-mt-zh-en)、[Xenova/opus-mt-en-zh](https://huggingface.co/Xenova/opus-mt-en-zh)。使用int8权重，来源提交、文件大小和SHA256固定在catalog.json；不把权重提交到Git。
- 运行库复用Windows安装包提供的ONNX Runtime 1.24.4，CPU执行、关闭遥测。中英方向首次使用时分别加载，支持贪心解码，不承诺术语翻译质量。
- 设置程序使用跨进程文件锁保护下载与安装，全部文件校验成功后换入用户模型目录；失败保留旧模型及恢复备份。
- Tokenizer从同版本source.spm读取实际归一化表；最多180个输入token一块，保留换行，未正常生成结束标记时返回失败，不输出截断译文。

## 验证入口

Windows Server 启动记录配置绝对路径、程序路径、配置来源与是否新建模板；找不到配置不能标作文件加载成功。热加载读取失败或配置临时消失时保留上一份有效配置，文件恢复后重新加载。日志仅包含错误类别，不打印 TOML 正文或密钥。翻译提供方未就绪时分别记录用户目录、运行库和线程错误；提供方已挂载不代表按需模型已完成加载。

`cargo test -p qingjian-core -p qingjian-offline -p qingjian-windows-server -p qingjian-windows-tsf -p qingjian-settings`。

Server的`--verify-translation <资源目录>`读取实际配置、运行固定公开样例，验证中英互译、多分句、提前确认、明确提交、取消和普通组句保护；不创建窗口、IPC连接、UIA或剪贴板访问，不挂在线提供方，也不写用户学习数据。此检查不等于实际宿主验收。

TSF集成测试native_translation默认忽略，需设置公开产品资源和测试模型目录；使用随机独立管道，不能连接日用服务。真实应用验收必须另行记录具体应用和操作，不把CLI推理或模拟读取当作实机兼容性结果。
