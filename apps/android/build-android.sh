#!/usr/bin/env bash
# 使用已有 Rust / JDK / SDK / NDK 构建并签名 ARM64 实验 APK，不安装工具。
set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
app="$repo/apps/android"
sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
[[ -n "$sdk" ]] || { echo '请设置 ANDROID_HOME 或 ANDROID_SDK_ROOT' >&2; exit 2; }
[[ -z "${JAVA_HOME:-}" ]] || export PATH="$JAVA_HOME/bin:$PATH"
for tool in javac java jar keytool zip; do
    command -v "$tool" >/dev/null || { echo "缺少构建工具：$tool" >&2; exit 2; }
done
bt="$sdk/build-tools/${QINGJIAN_ANDROID_BUILD_TOOLS:-35.0.0}"
android="$sdk/platforms/android-35/android.jar"
[[ -f "$android" ]] || { echo '请先安装 Android SDK platform android-35' >&2; exit 2; }
for tool in aapt2 d8 zipalign apksigner; do
    [[ -x "$bt/$tool" ]] || { echo "缺少 Android SDK 工具：$bt/$tool" >&2; exit 2; }
done

source "$app/version.properties"
name="${QINGJIAN_ANDROID_VERSION_NAME:-$versionName}"
code="${QINGJIAN_ANDROID_VERSION_CODE:-$versionCode}"
if [[ -z "${QINGJIAN_ANDROID_VERSION_NAME:-}" && "$name" == *-dev ]]; then
    short="$(git -C "$repo" rev-parse --short HEAD 2>/dev/null || echo unknown)"
    name="$name-$short"
    [[ -z "$(git -C "$repo" status --porcelain 2>/dev/null || true)" ]] || name="$name+"
fi
[[ "$code" =~ ^[1-9][0-9]*$ ]] || { echo 'versionCode 必须为正整数' >&2; exit 2; }
data="${QINGJIAN_ANDROID_DATA_DIR:-$repo/data/generated}"
for file in dict.qj glossary-en.qj; do
    [[ -s "$data/$file" ]] || { echo "缺少产品数据：$data/$file" >&2; exit 2; }
done

mkdir -p "$app/build/output"
stage="$(mktemp -d "$app/build/staging-XXXXXX")"
trap 'rm -rf -- "$stage"' EXIT
mkdir -p "$stage/assets/data" "$stage/classes" "$stage/generated" "$stage/dex" "$stage/lib/arm64-v8a"
cp -R "$app/src/main/assets/licenses" "$stage/assets/"
cp "$data/dict.qj" "$data/glossary-en.qj" "$stage/assets/data/"
[[ ! -s "$data/lm.qj" ]] || cp "$data/lm.qj" "$stage/assets/data/"

if [[ -n "${QINGJIAN_ANDROID_NATIVE_LIB:-}" ]]; then
    native="$QINGJIAN_ANDROID_NATIVE_LIB"
else
    QINGJIAN_ANDROID_NATIVE_OUT="$stage/native/arm64-v8a" bash "$app/build-native.sh"
    native="$stage/native/arm64-v8a/libqingjian_android.so"
fi
[[ -s "$native" ]] || { echo '未生成 ARM64 JNI 库' >&2; exit 2; }
cp "$native" "$stage/lib/arm64-v8a/libqingjian_android.so"

"$bt/aapt2" compile --dir "$app/src/main/res" -o "$stage/resources.zip"
"$bt/aapt2" link -o "$stage/base.apk" -I "$android" \
    --manifest "$app/src/main/AndroidManifest.xml" --java "$stage/generated" \
    --min-sdk-version 26 --target-sdk-version 35 --version-code "$code" --version-name "$name" \
    -A "$stage/assets" "$stage/resources.zip"
find "$app/src/main/java" "$stage/generated" -name '*.java' -print > "$stage/java-sources.txt"
javac -encoding UTF-8 --release 8 -classpath "$android" -d "$stage/classes" @"$stage/java-sources.txt"
jar cf "$stage/classes.jar" -C "$stage/classes" .
"$bt/d8" --release --min-api 26 --lib "$android" --output "$stage/dex" "$stage/classes.jar"
cp "$stage/dex/classes.dex" "$stage/classes.dex"
(cd "$stage" && zip -q base.apk classes.dex && zip -q -0 base.apk lib/arm64-v8a/libqingjian_android.so)
"$bt/zipalign" -P 16 -f 4 "$stage/base.apk" "$stage/aligned.apk"

keystore="${QINGJIAN_ANDROID_KEYSTORE:-$app/build/debug-signing.p12}"
alias="${QINGJIAN_ANDROID_KEY_ALIAS:-androiddebugkey}"
if [[ -z "${QINGJIAN_ANDROID_KEYSTORE:-}" ]]; then
    export QINGJIAN_ANDROID_STORE_PASSWORD=android QINGJIAN_ANDROID_KEY_PASSWORD=android
    if [[ ! -f "$keystore" ]]; then
        keytool -genkeypair -keystore "$keystore" -storetype PKCS12 \
            -storepass:env QINGJIAN_ANDROID_STORE_PASSWORD -keypass:env QINGJIAN_ANDROID_KEY_PASSWORD \
            -alias "$alias" -keyalg RSA -keysize 2048 -validity 3650 -dname 'CN=Android Debug'
    fi
else
    : "${QINGJIAN_ANDROID_STORE_PASSWORD:?请提供签名密码环境变量}"
    export QINGJIAN_ANDROID_KEY_PASSWORD="${QINGJIAN_ANDROID_KEY_PASSWORD:-$QINGJIAN_ANDROID_STORE_PASSWORD}"
fi
out="$app/build/output/jianci-${name//[^a-zA-Z0-9.+_-]/_}-arm64.apk"
"$bt/apksigner" sign --ks "$keystore" --ks-key-alias "$alias" \
    --ks-pass env:QINGJIAN_ANDROID_STORE_PASSWORD --key-pass env:QINGJIAN_ANDROID_KEY_PASSWORD \
    --out "$out" "$stage/aligned.apk"
"$bt/apksigner" verify --verbose "$out"
"$bt/zipalign" -c -P 16 4 "$out"
echo "APK：$out"
