#!/usr/bin/env bash
# 打包预检回归：失败时不编译、不清理已有应用，不使用本机产品数据。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/apps/macos/scripts" "$WORK/data/generated" "$WORK/bin" "$WORK/target/Qingjian.app"
# 系统安装探测也隔离到 fixture，不依赖测试机器上是否装过青简。
sed "s|/Library/Input Methods/|$WORK/system/|g" "$ROOT/apps/macos/scripts/bundle.sh" > "$WORK/apps/macos/scripts/bundle.sh"
printf '保留旧应用\n' > "$WORK/target/Qingjian.app/marker"
cat > "$WORK/bin/cargo" <<'MOCK'
#!/usr/bin/env bash
touch "$BUNDLE_TEST_WORK/cargo-called"
exit 73
MOCK
chmod +x "$WORK/bin/cargo"
export BUNDLE_TEST_WORK="$WORK"
export PATH="$WORK/bin:$PATH"
export QINGJIAN_TARGET=aarch64-apple-darwin
check() {
  local expected="$1" message="$2" status=0
  shift 2
  rm -f "$WORK/cargo-called"
  bash "$WORK/apps/macos/scripts/bundle.sh" "$@" > "$WORK/output" 2>&1 || status=$?
  if [[ "$status" != "$expected" ]]; then
    cat "$WORK/output"
    echo "预期退出 $expected，实际 $status" >&2
    exit 1
  fi
  [[ -f "$WORK/target/Qingjian.app/marker" ]]
  if [[ "$expected" == 73 ]]; then
    [[ -f "$WORK/cargo-called" ]]
  else
    [[ ! -e "$WORK/cargo-called" ]]
    grep -q "$message" "$WORK/output"
  fi
}
check 1 'tools/release/data-fetch.sh'
touch "$WORK/data/generated/dict.qj" "$WORK/data/generated/dict.tsv"
check 1 'tools/release/data-fetch.sh'
printf '正式 QJ 测试占位\n' > "$WORK/data/generated/dict.qj"
check 73 ''
rm "$WORK/data/generated/dict.qj"
printf '正式 TSV 测试占位\n' > "$WORK/data/generated/dict.tsv"
check 73 ''
mkdir -p "$WORK/system/Qingjian.app"
check 1 '系统目录已安装' --install
# 已有系统安装不阻止正常打包，只阻止再装一份用户副本。
check 73 ''
echo '打包预检：全部通过'
