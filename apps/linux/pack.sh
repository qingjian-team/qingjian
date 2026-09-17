#!/usr/bin/env bash
# 打一个自包含的青简 Linux 安装包(tarball)：内含 .so + conf + 数据 + install.sh。
# 解开后 `bash install.sh` 即装，不依赖 git 仓库，也不需要 rust 工具链。
# 用法：bash apps/linux/pack.sh   → 产物在 dist/qingjian-linux-<版本>.tar.gz
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
so="$repo/build/fcitx5-shim/libqingjian.so"
[[ -f $so ]] || {
    echo "先构建 .so:" >&2
    echo "  cargo build -p qingjian-linux-host --release" >&2
    echo "  cmake -S apps/linux/fcitx5-shim -B build/fcitx5-shim -DCMAKE_BUILD_TYPE=Release && cmake --build build/fcitx5-shim" >&2
    exit 1
}

version="$(grep -m1 '^version' "$repo/apps/linux/host/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
hash="$(git -C "$repo" rev-parse --short HEAD 2>/dev/null || echo nogit)"
name="qingjian-linux-${version}-${hash}"

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
payload="$stage/$name"
mkdir -p "$payload"/{conf,data}

# 产物 + 元数据
cp "$so" "$payload/libqingjian.so"
cp "$repo/apps/linux/install.sh" "$payload/install.sh"
cp "$repo/apps/linux/fcitx5-shim/conf/addon-qingjian.conf" "$payload/conf/"
cp "$repo/apps/linux/fcitx5-shim/conf/inputmethod-qingjian.conf" "$payload/conf/"
# 数据平铺进 data/（install.sh 分发模式从这里取）。
# 产品数据（data/generated，由 gh release download data 取回）优先；打包过的 .qj 在就不再带同名 tsv 样例源。
gen="$repo/data/generated"
put_first() { # put_first <候选源...>：第一个存在的拷进 data/
    local src
    for src in "$@"; do
        if [[ -f $src ]]; then
            cp "$src" "$payload/data/"
            return
        fi
    done
}
put_first "$gen/dict.qj"        "$repo/assets/lexicon/dict.tsv"
put_first "$gen/lm.qj"
put_first "$gen/english.tsv"    "$repo/assets/lexicon/english.tsv"
put_first "$gen/glossary-en.qj" "$repo/assets/glossary/glossary-en.tsv"
put_first "$gen/glossary-zh.qj" "$repo/assets/glossary/glossary-zh.tsv"
put_first "$gen/glossary-ja.qj" "$repo/assets/glossary/glossary-ja.tsv"
put_first "$repo/assets/glossary/glossary-es.tsv"
# emoji 表与词汇等级表（git 资产；统计页按级数词汇）
cp "$repo/assets/emoji/"emoji-*.tsv   "$payload/data/"
cp "$repo/assets/levels/"levels-*.tsv "$payload/data/"
# 随包领域词库（11 本，缺省只开成语，其余配置里开）；目录空时跳过（glob 不展开也不炸）
if [[ -d "$gen/dicts" ]]; then
    mkdir -p "$payload/data/dicts"
    for f in "$gen/dicts/"*.qj; do
        if [[ -f $f ]]; then
            cp "$f" "$payload/data/dicts/"
        fi
    done
fi
# 本地整句模型（可选）：data 发布资产 model.qjm 在就带上，没有就不重排。
if [[ -f "$repo/data/model/model.qjm" ]]; then
    cp "$repo/data/model/model.qjm" "$payload/data/"
fi

cat > "$payload/README.txt" <<EOF
青简输入法 Linux 端(Fcitx5)$version-$hash

安装：  bash install.sh
卸载：  见下
前提：  已装 fcitx5（本包只放用户目录，不动系统）

装完打开 fcitx5-configtool，把「青简」加进输入法列表即可。
配置文件在 ~/.config/qingjian/config.toml（改学习语言/模糊音，存盘即热生效）。

卸载：
  rm -f ~/.local/lib/fcitx5/libqingjian.so
  rm -f ~/.local/share/fcitx5/addon/qingjian.conf ~/.local/share/fcitx5/inputmethod/qingjian.conf
  rm -f ~/.config/environment.d/qingjian-fcitx5.conf
  （词库与学习数据在 ~/.local/share/qingjian，想彻底清也一并删）
  然后 fcitx5 -rd 重启
EOF

mkdir -p "$repo/dist"
out="$repo/dist/$name.tar.gz"
tar czf "$out" -C "$stage" "$name"
echo "安装包已生成：$out"
echo "大小：$(du -h "$out" | cut -f1)"
echo "分发后对方：tar xzf $name.tar.gz && cd $name && bash install.sh"
