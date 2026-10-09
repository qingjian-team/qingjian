#!/usr/bin/env bash
# 无 Android 设备的 JVM 回归：复制、键位、长按、候选几何与真实 View 触摸路径。
set -euo pipefail

app="$(cd "$(dirname "$0")" && pwd)"
[[ -z "${JAVA_HOME:-}" ]] || export PATH="$JAVA_HOME/bin:$PATH"
command -v javac >/dev/null
command -v java >/dev/null
mkdir -p "$app/build"
stage="$(mktemp -d "$app/build/tests-XXXXXX")"
trap 'rm -rf -- "$stage"' EXIT
src="$app/src/main/java/org/qingjian/android"
mkdir -p "$stage/data" "$stage/ui"
find "$app/tests" -name '*.java' -print > "$stage/data-sources.txt"
for type in DataInstaller KeyboardLayout KeySpec DeleteRepeater CandidateStrip; do
    echo "$src/$type.java" >> "$stage/data-sources.txt"
done
javac -encoding UTF-8 --release 8 -d "$stage/data" @"$stage/data-sources.txt"
for test in DataInstallerCheck KeyboardLayoutCheck DeleteRepeaterCheck CandidateStripCheck; do
    java -cp "$stage/data" "org.qingjian.android.$test"
done
find "$app/ui-tests/stubs" -name '*.java' -print > "$stage/ui-sources.txt"
for type in KeyboardView Candidate CandidateStrip KeyboardLayout KeySpec DeleteRepeater; do
    echo "$src/$type.java" >> "$stage/ui-sources.txt"
done
echo "$app/ui-tests/KeyboardScrollCheck.java" >> "$stage/ui-sources.txt"
javac -encoding UTF-8 --release 8 -d "$stage/ui" @"$stage/ui-sources.txt"
java -cp "$stage/ui" org.qingjian.android.KeyboardScrollCheck
