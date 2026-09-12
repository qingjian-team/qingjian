#!/bin/bash
# 回放扫参：设置文件每行一组 `--tune` 设置（空行 = 缺省），并行跑 `qingjian-cli --replay`，汇成 TSV。
#
#   tools/eval/sweep.sh <冻结日志> <配置文件> <设置文件> <输出 tsv>
#
# 配置文件用一份 `[predict] enabled = false` 的副本（云联想开着 CLI 会因没密钥退出，也别拿输入法的密钥跑批量）。
# 二进制缺省 target/release/qingjian-cli，可用 QINGJIAN_CLI 指定；QINGJIAN_JOBS 是并行数（缺省 4）。
set -euo pipefail
log=$1
config=$2
settings=$3
out=$4
bin=${QINGJIAN_CLI:-target/release/qingjian-cli}
jobs=${QINGJIAN_JOBS:-4}
runs=$(mktemp -d)

run_one() {
  local setting="$1"
  local tag=${setting:-default}
  local file="$runs/${tag//[\/,= ]/_}.txt"
  if [[ -z "$setting" ]]; then
    "$bin" --config "$config" --replay "$log" --misses 0 > "$file" 2>&1
  else
    "$bin" --config "$config" --replay "$log" --misses 0 --tune "$setting" > "$file" 2>&1
  fi
  # 词 / 整句 / 英文 三行：首选命中数 / 总数、不在候选、现在仍纠
  TAG=$tag perl -ne '
    if (/^(词|整句|英文)\s.*不在候选\s+(\d+).*命中数 (\d+) \/ (\d+)/) { $n=$1; $top{$n}=$3; $all{$n}=$4; $miss{$n}=$2 }
    if (/当时纠错生效 (\d+) 条，现在仍纠 (\d+)/) { $cor{$n}=$2 }
    END { print join("\t", $ENV{TAG}, map { ($top{$_}//0, $all{$_}//0, $miss{$_}//0, $cor{$_}//0) } qw(词 整句 英文)), "\n" }' "$file"
}
export -f run_one
export bin config log runs

printf 'tag\t词命中\t词总\t词不在\t词仍纠\t整句命中\t整句总\t整句不在\t整句仍纠\t英文命中\t英文总\t英文不在\t英文仍纠\n' > "$out"
tr '\n' '\0' < "$settings" | xargs -0 -P "$jobs" -I{} bash -c 'run_one "$1"' _ {} >> "$out"
rm -rf "$runs"
