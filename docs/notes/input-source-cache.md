# 沙盒应用的输入源缓存导致切换崩溃

## 现象与证据（2026-09-15）

macOS 26.6.2 上，安装青简 0.1.2 后，微信与企业微信在系统切换浮窗中选到青简时退出。企业微信 5.0.10 连续两次退出，微信也有同一崩溃信息。

- 系统日志：`*** CFRelease() called with NULL ***`。
- 企业微信主线程：`CoreFoundation → -[TUINSCursorUIController _selectCurrentInputSource] + 84 → moveTextInputMenuHUD: + 272`。
- 本机框架反汇编确认：按 ID 调用 `TISCopyInputSourceRefForInputSourceID` 后，未经判空便调用 `TSMSelectInputSource` 和 `CFRelease`。
- 浮窗的 `tsmEnabledInputSourceIDs` 包含 `app.qingjian.inputmethod`；两应用私有的 `com.apple.IntlDataCache.le.kbdx` 却早于青简安装，分别停留在 9 月 10 日、11 日，没有该 ID。

普通进程和新建沙盒进程直接查找均成功，不能用这个结果排除故障。把企业微信的 `.le` 与 `.le.kbdx` **成对复制**到隔离诊断应用的缓存目录后，首次查询稳定返回 NULL（退出码 1）；只复制 `.kbdx` 会使缓存头失效、触发系统重建，从而掩盖问题。备份移走两份旧缓存后，同一查询成功（退出码 0）。

这组对照验证了缓存陈旧；保留原来的输入源 ID 和 Info.plist 即可恢复，不需要改输入协议或其他应用的签名。

## 修复边界

`apps/macos/scripts/repair-input-cache.sh` 缺省只处理 `com.tencent.xinWeChat` 与 `com.tencent.WeWorkMac`，根目录由 `getconf DARWIN_USER_CACHE_DIR` 定位。

1. 校验 bundle identifier，拒绝路径穿越与缓存符号链接。
2. 已含青简的键盘缓存不动；无缓存或缺少配对文件时交给系统重建。
3. 将过期的两份文件移动至同目录唯一的 `qingjian-input-cache-backup.*` 子目录。第二份移动失败时还原第一份。
4. 应用重启后由系统重建。工具不退出应用，也不处理聊天记录、登录信息或其他缓存。

开发安装与 pkg 安装均调用此工具；pkg 在当前登录用户的输入源注册成功后执行。故障修复失败只提示手工处理，不把输入源注册成功误报为失败。工具亦随包放在 `Contents/Resources/`。

回滚需先退出对应应用，再把工具输出的备份目录中的两个文件移回该应用缓存目录；若已有新缓存，先另行备份。旧缓存本身不包含青简，恢复后可能重新触发原故障。

## 验证

```bash
bash apps/macos/scripts/test-repair-input-cache.sh
clang -fobjc-arc -framework AppKit -framework Carbon \
  apps/macos/scripts/check-input-source.m -o target/check-input-source
target/check-input-source
```

脚本测试覆盖备份内容、幂等、已更新缓存、无关文件保留、非法路径和符号链接拒绝。`check-input-source.m` 仅是诊断程序：使用发生崩溃的系统查询接口，不切换输入源、不发送按键；私有接口不在当前系统上时返回 2，产品不依赖它。

要重放旧缓存，需把诊断程序包装成有独立 bundle identifier、启用 `com.apple.security.app-sandbox` 的临时 `.app`，将捕获的两份旧缓存放到其私有缓存目录，再运行诊断程序。不得把旧缓存写回真实聊天应用作为测试。

本次已在真实微信、企业微信均退出时备份移走旧缓存并重新打开应用；两个应用都生成了含青简的新缓存。真实聊天框的切换与输入仍需手工验收，自动化受辅助功能权限限制。
