#!/usr/bin/env bash
# 验证修复工具只移动过期缓存、保留可恢复备份，并拒绝越界目标。
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/qingjian-cache-test.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
app="$fixture/com.example.chat"
mkdir -p "$app"
printf 'international fixture' > "$app/com.apple.IntlDataCache.le"
printf 'old input source list' > "$app/com.apple.IntlDataCache.le.kbdx"
printf 'unrelated file' > "$app/messages.db"
bash "$script_dir/repair-input-cache.sh" --cache-root "$fixture" com.example.chat
[[ ! -e "$app/com.apple.IntlDataCache.le" && ! -e "$app/com.apple.IntlDataCache.le.kbdx" ]]
backups=("$app"/qingjian-input-cache-backup.*)
[[ ${#backups[@]} -eq 1 ]]
[[ "$(<"${backups[0]}/com.apple.IntlDataCache.le")" == 'international fixture' ]]
[[ "$(<"${backups[0]}/com.apple.IntlDataCache.le.kbdx")" == 'old input source list' ]]
[[ "$(<"$app/messages.db")" == 'unrelated file' ]]
bash "$script_dir/repair-input-cache.sh" --cache-root "$fixture" com.example.chat
printf 'international fixture' > "$app/com.apple.IntlDataCache.le"
printf 'app.qingjian.inputmethod' > "$app/com.apple.IntlDataCache.le.kbdx"
bash "$script_dir/repair-input-cache.sh" --cache-root "$fixture" com.example.chat
[[ -f "$app/com.apple.IntlDataCache.le" && -f "$app/com.apple.IntlDataCache.le.kbdx" ]]
backups=("$app"/qingjian-input-cache-backup.*)
[[ ${#backups[@]} -eq 1 ]]
if bash "$script_dir/repair-input-cache.sh" --cache-root "$fixture" ../com.example.chat; then
  echo "错误：未拒绝越界路径" >&2
  exit 1
fi
ln -s "$app" "$fixture/com.example.link"
if bash "$script_dir/repair-input-cache.sh" --cache-root "$fixture" com.example.link; then
  echo "错误：未拒绝符号链接" >&2
  exit 1
fi
mkdir -p "$fixture/com.example.filelink"
ln -s "$app/com.apple.IntlDataCache.le.kbdx" "$fixture/com.example.filelink/com.apple.IntlDataCache.le.kbdx"
if bash "$script_dir/repair-input-cache.sh" --cache-root "$fixture" com.example.filelink; then
  echo "错误：未拒绝缓存文件符号链接" >&2
  exit 1
fi
mkdir -p "$fixture/com.example.partial"
printf 'old input source list' > "$fixture/com.example.partial/com.apple.IntlDataCache.le.kbdx"
bash "$script_dir/repair-input-cache.sh" --cache-root "$fixture" com.example.partial
[[ -f "$fixture/com.example.partial/com.apple.IntlDataCache.le.kbdx" ]]
echo "缓存修复测试通过。"
