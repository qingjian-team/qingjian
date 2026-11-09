#!/usr/bin/env bash
# 在 Gitee 镜像上建与 GitHub 同名的发行版，标题、说明（CHANGELOG 那一节加平台提示）、预发布照搬 GitHub Release；不传安装包，下载仍走 GitHub 与网盘。
#
#   tools/release/gitee-release.sh <标签>...    # 例如 v0.3.3
#   tools/release/gitee-release.sh --all        # 补齐 GitHub 上所有已发布的版本（data-v* 不算）
#
# 要 GITEE_TOKEN（对 Gitee 仓库有推送权限的私人令牌）。标签由 Gitee 镜像从 GitHub 同步过去，标签还没到就先去 Gitee 仓库页点同步再跑。
# 已有同名发行版就改成 GitHub 现在的样子，重复跑没有副作用。
set -euo pipefail

REPO="qingjian-team/qingjian"
API="https://gitee.com/api/v5/repos/$REPO"

: "${GITEE_TOKEN:?要设 GITEE_TOKEN}"
for tool in gh jq curl; do
  command -v "$tool" >/dev/null || { echo "要装 ${tool}" >&2; exit 1; }
done

if [[ "${1:-}" == "--all" ]]; then
  # 按 GitHub 的发布时间从旧到新建：Gitee 把最后建的当成最新版。macOS 自带的 bash 3.2 没有 mapfile
  TAGS=()
  while IFS= read -r tag; do TAGS+=("$tag"); done < <(
    gh release list -R "$REPO" --limit 200 --exclude-drafts --json tagName,publishedAt \
      --jq 'sort_by(.publishedAt) | .[].tagName' |
      grep -E '^((macos|windows|linux|android)-)?v[0-9]')
else
  TAGS=("$@")
fi
[[ ${#TAGS[@]} -gt 0 ]] || { echo "用法：$0 <标签>... | --all" >&2; exit 1; }

# Gitee 的接口出错时也可能回 200 加 {"message": ...}，统一按有没有 message 判断；
# 字段一律 --form-string，说明以 @ 或 < 开头时 -F 会当成读文件
gitee() {
  local method="$1" path="$2"; shift 2
  local out
  out=$(curl -sS -X "$method" "$API$path" --form-string "access_token=$GITEE_TOKEN" "$@")
  if jq -e 'type == "object" and has("message")' <<<"$out" >/dev/null 2>&1; then
    echo "Gitee $method ${path}：$(jq -r .message <<<"$out")" >&2
    return 1
  fi
  printf '%s' "$out"
}

for tag in "${TAGS[@]}"; do
  release=$(gh release view "$tag" -R "$REPO" --json name,body,isPrerelease,isDraft)
  if [[ $(jq -r .isDraft <<<"$release") == true ]]; then
    echo "${tag} 在 GitHub 上还是草稿，跳过" >&2
    continue
  fi
  name=$(jq -r .name <<<"$release")
  body=$(jq -r .body <<<"$release")
  prerelease=$(jq -r .isPrerelease <<<"$release")
  commit=$(gh api "repos/$REPO/commits/$tag" --jq .sha)

  # GET 不带表单，令牌放查询串
  existing=$(curl -sS "$API/releases/tags/$tag?access_token=$GITEE_TOKEN" | jq -r 'if type == "object" then .id // empty else empty end')
  fields=(--form-string "tag_name=$tag" --form-string "name=$name" --form-string "body=$body" --form-string "prerelease=$prerelease")
  if [[ -n "$existing" ]]; then
    gitee PATCH "/releases/$existing" "${fields[@]}" >/dev/null
    echo "${tag}：已更新 https://gitee.com/$REPO/releases/tag/$tag"
  else
    gitee POST "/releases" "${fields[@]}" --form-string "target_commitish=$commit" >/dev/null
    echo "${tag}：已创建 https://gitee.com/$REPO/releases/tag/$tag"
  fi
done
