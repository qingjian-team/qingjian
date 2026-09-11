#!/usr/bin/env bash
# 发版最后一步：从 CHANGELOG.md + GitHub Releases API + 各版本的 SHA256SUMS / build-info.json 生成 releases.json，
# 挂到本次发布上，并覆盖到 GitHub 的 latest 发布上（官网固定取 releases/latest/download/releases.json，
# 而 latest 不一定是本次：各平台版本号独立，Windows 内测版发出去时 latest 可能仍是 macOS 的那版）。
#
#   tools/release/publish-releases-json.sh <本次发布的标签> <输出目录>
#
# 需要 gh（带仓库写权限）与 python3；Windows runner 上 python3 叫 python，用 PYTHON 环境变量指定。
set -euo pipefail

TAG="$1"
OUT_DIR="$2"
PYTHON="${PYTHON:-python3}"
REPO="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY 未设}"
TMP="$(mktemp -d)"

gh api "repos/$REPO/releases" --paginate > "$TMP/releases.api.json"
META="$TMP/meta"
for tag in $(gh release list --limit 100 --json tagName --jq '.[].tagName' | grep -E '^(macos-|windows-|linux-)?v[0-9]'); do
  mkdir -p "$META/$tag"
  gh release download "$tag" --pattern SHA256SUMS --pattern build-info.json --dir "$META/$tag" || true
done
mkdir -p "$OUT_DIR"
"$PYTHON" tools/release/releases_json.py --repo "$REPO" \
  --api-json "$TMP/releases.api.json" --meta-dir "$META" --out "$OUT_DIR/releases.json"
cat "$OUT_DIR/releases.json"

gh release upload "$TAG" "$OUT_DIR/releases.json" --clobber
LATEST="$(gh release view --json tagName --jq .tagName)"
if [[ "$LATEST" != "$TAG" ]]; then
  echo "latest 是 $LATEST，也覆盖一份 releases.json 上去"
  gh release upload "$LATEST" "$OUT_DIR/releases.json" --clobber
fi
