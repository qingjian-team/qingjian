#!/usr/bin/env bash
# 让官网（qingjian-team/qingjian-web，Cloudflare Workers Builds 按仓库提交自动构建）重新构建：
# 往它的 src/content/upstream.json 写一笔「主仓库现在的版本标签 / 文档提交」并推一个提交。
# 官网构建时会拉最新 Release 的 releases.json 与主仓库 docs/user，所以只要有提交就够了。
#
# 用法：tools/release/bump-website.sh <release|docs>
#   release  发版后调用，记 GITHUB_REF_NAME（标签）与 GITHUB_SHA
#   docs     docs/user 改动后调用，只更新文档提交号
# 需要环境变量 QINGJIAN_WEB_TOKEN：对 qingjian-web 有 Contents: write 的 fine-grained PAT。
set -euo pipefail

kind=${1:?用法: bump-website.sh <release|docs>}
token=${QINGJIAN_WEB_TOKEN:?缺 QINGJIAN_WEB_TOKEN}
repo=${QINGJIAN_WEB_REPO:-qingjian-team/qingjian-web}
sha=${GITHUB_SHA:-$(git rev-parse HEAD)}
tag=${GITHUB_REF_NAME:-}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git clone -q --depth 1 "https://x-access-token:${token}@github.com/${repo}.git" "$work/web"
cd "$work/web"

file=src/content/upstream.json
mkdir -p "$(dirname "$file")"
current_release=$(python3 -c "import json,sys; print(json.load(open('$file')).get('release',''))" 2>/dev/null || true)
case "$kind" in
  release) release=$tag; message="同步主仓库：发版 ${tag}" ;;
  docs) release=$current_release; message="同步主仓库：用户文档 ${sha:0:7}" ;;
  *) echo "未知类型: $kind" >&2; exit 2 ;;
esac
python3 - "$file" "$release" "$sha" <<'PY'
import json, sys, datetime
path, release, sha = sys.argv[1:4]
json.dump(
    {"release": release, "docs": sha, "updated": datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")},
    open(path, "w", encoding="utf-8"),
    ensure_ascii=False,
    indent=2,
)
open(path, "a", encoding="utf-8").write("\n")
PY
git add "$file"
if git diff --cached --quiet; then
  echo "官网仓库无需更新"
  exit 0
fi
git -c user.name="qingjian-ci" -c user.email="ci@qingjian.app" commit -q -m "$message" -m "主仓库提交 ${sha}"
git push -q origin HEAD
echo "已推送到 ${repo}：${message}"
