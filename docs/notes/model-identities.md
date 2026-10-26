# 含章模型名称与检查点

「含章」是青简本地神经模型系列。下面的检查点编号来自现有模型配置，用于追溯权重；两款模型尚未指定正式发行版本。应用版本和数据 Release 标签也不充当模型版本。

| 模型 | 英文名 | 标识 | 用途 | 当前检查点 |
| --- | --- | --- | --- | --- |
| 含章·通变 | Hanzhang Tongbian — neural pinyin decoding and correction | `hanzhang-tongbian` | 拼音到汉字解码、纠错、整句候选重排与词图未覆盖时的生成 | `small-15000` |
| 含章·知微 | Hanzhang Zhiwei — context-aware candidate rescoring | `hanzhang-zhiwei` | 依据光标前文给整句候选重排；没有通变时作为回退 | `small-155478` |

`tools/release/pack-model.sh` 把正式名称写进 `.qjm` 的 `name` 元数据，`version` 暂记型号 `hanzhang-tongbian-small` / `hanzhang-zhiwei-small`，不附检查点编号。文件名与型号相同，分别放在 `models/hanzhang-tongbian/` 与 `models/hanzhang-zhiwei/`；检查点仍记录在上表和原始配置中，数据 Release 用独立的 `data-vN` 标签和 SHA-256 锁定具体字节。

改名称或元数据时，两份模型都要重新打包并重算哈希；不能只改脚本而沿用旧 `.qjm`。正式发布时将两份模型和词库装进同一个运行时压缩包，并更新 `tools/release/data.lock`。
