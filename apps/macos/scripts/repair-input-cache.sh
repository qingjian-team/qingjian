#!/usr/bin/env bash
# 修复沙盒应用未刷新输入源列表导致的切换崩溃；只备份指定应用的两份键盘缓存。
# 缺省处理微信和企业微信。可传其他 bundle identifier；--cache-root 供离线诊断和测试使用。
set -euo pipefail

usage() {
  echo "用法：$0 [--cache-root 目录] [应用 bundle identifier ...]"
  echo "缺省处理 com.tencent.xinWeChat 和 com.tencent.WeWorkMac；保留备份，不退出应用。"
}

cache_root=""
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi
if [[ "${1:-}" == "--cache-root" ]]; then
  if [[ $# -lt 2 || -z "$2" ]]; then
    usage >&2
    exit 2
  fi
  cache_root="$2"
  shift 2
else
  cache_root="$(getconf DARWIN_USER_CACHE_DIR)"
fi
if [[ ! -d "$cache_root" ]]; then
  echo "缓存根目录不存在：$cache_root" >&2
  exit 2
fi
if [[ $# -eq 0 ]]; then
  set -- com.tencent.xinWeChat com.tencent.WeWorkMac
fi

# 先校验全部参数，避免处理一半后才发现非法目标。
for bundle_id in "$@"; do
  if [[ ! "$bundle_id" =~ ^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)+$ ]]; then
    echo "无效的应用 bundle identifier：$bundle_id" >&2
    exit 2
  fi
done

for bundle_id in "$@"; do
  cache_dir="${cache_root%/}/$bundle_id"
  keyboard="$cache_dir/com.apple.IntlDataCache.le.kbdx"
  international="$cache_dir/com.apple.IntlDataCache.le"
  if [[ -L "$cache_dir" || -L "$keyboard" || -L "$international" ]]; then
    echo "拒绝处理符号链接缓存：$bundle_id" >&2
    exit 2
  fi
  if [[ ! -f "$keyboard" ]]; then
    echo "${bundle_id}：没有旧键盘缓存，无需处理。"
    continue
  fi
  # 缓存中已登记青简就不动，反复执行不会产生多份备份。
  # 使用二进制匹配，不输出缓存内容；键盘缓存里的输入源 ID 是 ASCII。
  if LC_ALL=C grep -aFq 'app.qingjian.inputmethod' "$keyboard"; then
    echo "${bundle_id}：键盘缓存已包含青简，无需处理。"
    continue
  else
    status=$?
    if [[ "$status" -ne 1 ]]; then
      echo "${bundle_id}：无法读取键盘缓存，未修改。" >&2
      exit 1
    fi
  fi
  if [[ ! -f "$international" ]]; then
    echo "${bundle_id}：缓存不完整，系统会自行重建，无需处理。"
    continue
  fi
  backup="$(mktemp -d "$cache_dir/qingjian-input-cache-backup.XXXXXX")"
  mv "$international" "$backup/com.apple.IntlDataCache.le"
  if ! mv "$keyboard" "$backup/com.apple.IntlDataCache.le.kbdx"; then
    # 第二份移动失败时还原第一份，不留下半套缓存。
    mv "$backup/com.apple.IntlDataCache.le" "$international"
    echo "${bundle_id}：备份失败，已还原第一份缓存。" >&2
    exit 1
  fi
  echo "${bundle_id}：旧键盘缓存已备份到 $backup"
  echo "请完全退出并重新打开该应用，macOS 将重新建立输入源列表。"
done
