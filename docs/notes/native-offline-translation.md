# 原生离线选区翻译

Windows 选区翻译在现有 Server 进程中执行本地 ONNX 推理；不依赖 Python、HTTP 服务或 API 密钥。首次使用在「设置 → 实验功能」明确下载中英双向模型，下载后断网可用。

## 数据流与取消

`选区快捷键 → TSF 读取 → Router → Engine::request_translation → NativePredictor → CPU ONNX → Engine::poll_prediction → 译文候选 → 用户确认`。

Core 独立的 `translation_predictor` 只接显式选区请求。普通候选联想、停键翻译和释义仍走原来的提供方。配置选择本地翻译后，模型或目标语言不支持时返回失败，不自动转在线服务。私密输入不提交；取消和失焦使请求代次过期，迟到结果无法写入新组句。

只维护一个推理线程和一个待发请求槽，后来的请求覆盖尚未执行的请求。解码逐步检查取消与60秒任务期限；ONNX单次运行不能强制中断，但在后台执行，不阻塞输入线程。关闭时取消并唤醒工作线程。

## 选区读取与确认

DLL 只在显式触发时读取应用通过 TSF 提供的非空白选区，上限为 500 个 UTF-16 单元。私密输入、没有上下文、编辑会话被拒绝或无法读取时返回空选区，Server 收到后结束请求。

DLL 保存 TSF 选区的原范围、原文与焦点。用户确认译文后，回填前重新校验文档、前台窗口、焦点、选区位置和原文。

协议 10 的配套 DLL 和 Server 支持在译文未就绪时确认并继续等待，译文出现后须再次确认。升级后应重开宿主应用。

## 模型来源与安装

- 原始模型：[Helsinki-NLP/opus-mt-zh-en](https://huggingface.co/Helsinki-NLP/opus-mt-zh-en)、[Helsinki-NLP/opus-mt-en-zh](https://huggingface.co/Helsinki-NLP/opus-mt-en-zh)，作者为赫尔辛基大学 Language Technology Research Group，中译英采用CC-BY-4.0，英译中采用Apache-2.0；以各原始模型卡为准。
- ONNX导出：[Xenova/opus-mt-zh-en](https://huggingface.co/Xenova/opus-mt-zh-en)、[Xenova/opus-mt-en-zh](https://huggingface.co/Xenova/opus-mt-en-zh)。使用int8权重，来源提交、文件大小和SHA256固定在catalog.json；不把权重提交到Git。
- 运行库复用Windows安装包提供的ONNX Runtime 1.24.4，CPU执行、关闭遥测。中英方向首次使用时分别加载，支持贪心解码，不承诺术语翻译质量。
- 设置程序使用跨进程文件锁保护下载与安装，全部文件校验成功后换入用户模型目录；失败保留旧模型及恢复备份。
- Tokenizer从同版本source.spm读取实际归一化表；安装归一化器失败时返回推理错误，不继续用不完整的分词配置。最多180个输入token一块，保留换行，未正常生成结束标记时返回失败，不输出截断译文。

## 验证入口

Windows Server 启动记录配置绝对路径、程序路径、配置来源与是否新建模板；找不到配置不能标作文件加载成功。热加载读取失败或配置临时消失时保留上一份有效配置，文件恢复后重新加载。日志仅包含错误类别，不打印 TOML 正文或密钥。翻译提供方未就绪时分别记录用户目录、运行库和线程错误；提供方已挂载不代表按需模型已完成加载。

`cargo test -p qingjian-core -p qingjian-offline -p qingjian-windows-server -p qingjian-windows-tsf -p qingjian-settings`。

Server 的 `--verify-translation <资源目录>` 读取实际配置，运行固定公开样例，验证中英互译、多分句、提前确认、明确提交、取消和普通组句保护。检查使用无窗口输出端，不建立 IPC 连接、不挂在线提供方，也不写用户学习数据。

TSF 客户端管道集成测试 `native_translation` 默认忽略，需设置公开产品资源和测试模型目录，使用带进程编号的独立管道。宿主选区读取和回填需在实际应用中验证，记录具体应用与操作。
