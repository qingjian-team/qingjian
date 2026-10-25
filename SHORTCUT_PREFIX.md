## 快捷短语模式键配置支持

在 ModeKeys 结构体中添加了 `shortcut` 字段和 `SHORTCUT_PREFIX` 常量，
用于配置快捷短语模式键（缺省为 `i`）。

### 变更文件
- `crates/qingjian-core/src/engine/mode_keys.rs`
- `crates/qingcore/src/engine/composing.rs`  
- `crates/qing-core/src/engine/mod.rs`

### 代码改动
- 新增 SHORTCUT_PREFIX 常量：定义为 'i'
- 在 ModeKeys 结构体中添加 shortcut 字段并设置默认值为 'i'
- 在 is_valid() 方法中增加 shortcut != self.question 的校验条件
- 在 is_shortcut() 方法添加快捷短语模式检测
- 在 shortcut_body() 方法添加快捷短语模式下前缀提取
- 在 letterless() 方法中将 shortcut 设为 '\0'
- 新增对应的测试用例：shortcut_mode_is_detected、shortcut_mode_shifted此行：快捷短语模式不支持三个键值
- 在 extras.rs 添加 has_shortcut_phrase 和 shortcut_phrase_candidates 方法（#[allow(dead_code)]用于未来实现）
- 更新 extras.rs 中的 insert_custom_phrases 方法支持快捷短语模式优先显示

### 测试
新增快捷短语模式相关的测试，确保默认值、双拼支持、重复键值等场景

### 下一步
平台层需要：
1. 配置界面支持快捷短语键选择
2. 在输入法壳中处理快捷短语按键事件
3. 设置程序支持用户自定义短语管理

### 参考
docs/plan/todo.md 第54行：「快捷短语（`前缀）- 编码 → 短语表，用户可增删