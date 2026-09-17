#!/usr/bin/env bash
# 青简 Linux 端用户级安装：不动系统目录，全部落用户目录。
# 双模式（安装逻辑单一真相源）：
#   开发模式 = 在 git 仓库里跑(bash apps/linux/install.sh)，从 build/ 与 assets/ 取。
#   分发模式 = 在解开的安装包里跑(bash install.sh)，从脚本同目录的 libqingjian.so/conf/data 取。
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$here/libqingjian.so" ]]; then
    # 分发模式：payload 就在脚本旁。
    so="$here/libqingjian.so"
    src_conf="$here/conf"
    src_data="$here/data"
else
    # 开发模式：回到仓库根。
    repo="$(cd "$here/../.." && pwd)"
    so="$repo/build/fcitx5-shim/libqingjian.so"
    [[ -f $so ]] || { echo "先构建：cargo build -p qingjian-linux-host --release && cmake -S apps/linux/fcitx5-shim -B build/fcitx5-shim && cmake --build build/fcitx5-shim" >&2; exit 1; }
    src_conf="$repo/apps/linux/fcitx5-shim/conf"
    src_data="$repo/assets"  # 开发模式数据分散在 assets/lexicon 与 assets/glossary，下面单独处理
fi

lib_dir="$HOME/.local/lib/fcitx5"
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/fcitx5"
install -Dm755 "$so" "$lib_dir/libqingjian.so"
install -Dm644 "$src_conf/addon-qingjian.conf" "$data_dir/addon/qingjian.conf"
install -Dm644 "$src_conf/inputmethod-qingjian.conf" "$data_dir/inputmethod/qingjian.conf"

# 数据文件：全量词库 + 各语种释义表 + 英文词表 + 模型。分两层（fcitx5 StandardPaths 同款语义）：
#   $qdata/dist/ = 随包层，安装器独占，每次安装整个换新——升级就是覆盖这里；
#   $qdata 根    = 用户层，学习数据(user*.tsv/usage.tsv)与用户自有的同名覆盖件，安装器不碰。
# host 查找时用户层盖过随包层，所以「用户自己的文件优先」不再靠「装机不覆盖」实现。
qdata="${XDG_DATA_HOME:-$HOME/.local/share}/qingjian"
# 先装进暂存层，全部就绪后再原子换名——坏包/缺数据时不打烂上一版随包层。
dist_new="$qdata/dist.new"
rm -rf "$dist_new"
mkdir -p "$dist_new"
put_data() { # put_data <目标文件名> <开发模式源相对 assets 的路径>；哪儿都没有就跳过（缺了对应功能降级）
    local name="$1" dev_rel="$2" src
    local sources=("$src_data/$name" "$src_data/$dev_rel" "${repo:-/nonexistent}/data/generated/$name")
    # 同名 .qj 已进随包层就不再装 tsv 样例源（host 查找也是 .qj 优先）。
    if [[ $name != *.tsv || ! -f "$dist_new/${name%.tsv}.qj" ]]; then
        for src in "${sources[@]}"; do
            if [[ -f $src ]]; then
                install -Dm644 "$src" "$dist_new/$name"
                break
            fi
        done
    fi
}
put_data dict.qj         lexicon/dict.qj          # 产品词库（.qj 优先于 tsv）
put_data dict.tsv        lexicon/dict.tsv
put_data lm.qj           lexicon/lm.qj            # 整句语言模型；缺了退化一元词频
put_data english.tsv     lexicon/english.tsv
put_data glossary-en.qj  glossary/glossary-en.qj
put_data glossary-en.tsv glossary/glossary-en.tsv
put_data glossary-zh.qj  glossary/glossary-zh.qj
put_data glossary-zh.tsv glossary/glossary-zh.tsv
put_data glossary-ja.qj  glossary/glossary-ja.qj
put_data glossary-ja.tsv glossary/glossary-ja.tsv
put_data glossary-es.tsv glossary/glossary-es.tsv
put_data emoji-zh.tsv    emoji/emoji-zh.tsv       # emoji 候选
put_data emoji-en.tsv    emoji/emoji-en.tsv
put_data levels-en.tsv   levels/levels-en.tsv     # 词汇等级（统计）
put_data levels-ja.tsv   levels/levels-ja.tsv
# 随包领域词库（11 本，缺省只开成语，其余在配置 [dictionaries] domains 里开）；
# 用户自己的词库放 user-dicts/，不在此列。
for dicts_src in "$src_data/dicts" "${repo:-/nonexistent}/data/generated/dicts"; do
    if [[ -d $dicts_src ]]; then
        for f in "$dicts_src/"*.qj; do
            [[ -f $f ]] || continue
            install -Dm644 "$f" "$dist_new/dicts/$(basename "$f")"
        done
        break
    fi
done
# 本地整句模型（可选）：随包的落 dist/model/，升级跟着包走；没带就不重排。
# 用户自己的 .qjm 放 $qdata/model/，查找时盖过随包的。
for model_src in "$src_data/model.qjm" "${repo:-/nonexistent}/data/model/model.qjm"; do
    if [[ -f $model_src ]]; then
        install -Dm644 "$model_src" "$dist_new/model/model.qjm"
        break
    fi
done

# 词库守卫：按装完后实际生效的随包层判——暂存层非空即将换层，
# 词库必须在暂存层或用户层；暂存层为空不换层，上一版随包层 dist/ 里的也算数。
if [[ -n "$(ls -A "$dist_new")" ]]; then
    dict_layer="$dist_new"
else
    dict_layer="$qdata/dist"
fi
[[ -f "$dict_layer/dict.qj" || -f "$dict_layer/dict.tsv" || -f "$qdata/dict.qj" || -f "$qdata/dict.tsv" ]] \
    || { rm -rf "$dist_new"; echo "词库缺失：分发包 data/ 或仓库里都找不到 dict.qj/dict.tsv（上一版随包数据未动）" >&2; exit 1; }

# 随包层全部就绪：先把旧层挪开、新层就位之后才删。
# 两步换名之间无论何时被杀都不丢数据（旧层在 dist.old、新层在 dist.new，重跑安装即恢复）。
# 本包一无所带（坏包/被裁剪）时不换：上一版随包层（含 56MB 模型）保留，只警告。
if [[ -n "$(ls -A "$dist_new")" ]]; then
    rm -rf "${qdata:?}/dist.old"
    [[ ! -d "$qdata/dist" ]] || mv "$qdata/dist" "$qdata/dist.old"
    mv "$dist_new" "$qdata/dist"
    rm -rf "${qdata:?}/dist.old"
else
    rmdir "$dist_new"
    echo "注意：本包不带任何数据文件，保留上一版随包层 $qdata/dist 不动。" >&2
fi

# 分层可见化：用户层同词干文件盖过随包数据（升级对它们不生效）——
# 静默遮蔽最伤人，点名列出。
shadow=""
[[ -d "$qdata/dist" ]] && while IFS= read -r f; do
    rel="${f#"$qdata/dist/"}"
    case "$rel" in
    dicts/*) continue ;; # 领域词库的用户层是 user-dicts/，根下无遮蔽关系
    model/*)
        if [[ -f "$qdata/$rel" ]]; then shadow+="$qdata/$rel"$'\n'; fi
        continue
        ;;
    esac
    stem="${rel%.*}"
    for cand in "$stem.qj" "$stem.tsv"; do
        if [[ -f "$qdata/$cand" ]]; then shadow+="$qdata/$cand"$'\n'; fi
    done
done < <(find "$qdata/dist" -type f)
if [[ -n $shadow ]]; then
    echo "注意：用户层下列文件将盖过随包同名数据（随包升级对它们不生效）。"
    echo "  自己放的覆盖件属正常；若非你放置，请删除："
    printf '%s' "$shadow" | sort -u | sed 's/^/    /'
fi

# 播种配置文件（已存在则不动）：模糊音开常用四路，其余键列全供随手改；改完敲下个键即热生效。
qj_config="$HOME/.config/qingjian/config.toml"
if [[ ! -f $qj_config ]]; then
    mkdir -p "$(dirname "$qj_config")"
    cat > "$qj_config" <<'QJCONF'
# 青简配置。保存后敲下一个键即生效（热加载），不用重启。
[general]
learning_language = "en"   # 学习语言：en 英语 / ja 日语 / es 西班牙语
page_size = 9              # 每页候选数，1-9

[fuzzy]
z_zh = true
c_ch = true
s_sh = true
n_l = true
f_h = false
l_r = false
an_ang = false
en_eng = false
in_ing = false
QJCONF
fi

# .so 的搜索路径要靠 FCITX_ADDON_DIRS（conf 文件用户目录原生支持，不用它）。
# 写进 environment.d 供下次登录；本次立即生效靠下面带环境变量重启。
env_file="$HOME/.config/environment.d/qingjian-fcitx5.conf"
mkdir -p "$(dirname "$env_file")"
echo "FCITX_ADDON_DIRS=$lib_dir:/usr/lib/fcitx5" > "$env_file"

echo "已安装：$lib_dir/libqingjian.so + addon/inputmethod conf + 词库数据"
# 延迟重启 + setsid 脱离终端：立即替换 fcitx5 会吞掉启动本脚本那次回车的松键事件，
# Wayland 合成器会当成回车一直按着 → 终端被无限回车（2026-09-15 实测）。
# sleep 给松键留窗口；setsid 让新 fcitx5 不当终端的子进程。
setsid bash -c "sleep 0.5; FCITX_ADDON_DIRS='$lib_dir:/usr/lib/fcitx5' exec fcitx5 -rd" >/dev/null 2>&1 </dev/null &
echo "fcitx5 将在半秒后带新插件重启；在输入法配置里添加「青简」即可(fcitx5-configtool)。"
