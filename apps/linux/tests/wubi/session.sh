#!/usr/bin/env bash
# //! 私有 D-Bus 与 Xvfb 中运行真实五笔输入，不使用桌面配置或学习数据。
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
if [[ ${1:-} != --private ]]; then
    export QINGJIAN_WUBI_TEST_ROOT
    QINGJIAN_WUBI_TEST_ROOT=$(mktemp -d /tmp/qj-wubi-XXXXXX)
    cleanup_directory() {
        if mountpoint -q "$QINGJIAN_WUBI_TEST_ROOT/runtime/doc"; then
            fusermount3 -u "$QINGJIAN_WUBI_TEST_ROOT/runtime/doc"
        fi
        if [[ ${KEEP_TEST_DATA:-0} == 1 ]]; then
            printf 'Test data: %s\n' "$QINGJIAN_WUBI_TEST_ROOT"
        else
            rm -rf --one-file-system "$QINGJIAN_WUBI_TEST_ROOT"
        fi
    }
    trap cleanup_directory EXIT
    dbus-run-session -- "$0" --private "${1:-fcitx5}"
    exit
fi
FRONTEND=${2:?fcitx5, fcitx4 or ibus}
TEST_ROOT=${QINGJIAN_WUBI_TEST_ROOT:?Run through the private session wrapper}
PIDS=()
cleanup() {
    for process in "${PIDS[@]}"; do kill "$process" 2>/dev/null || true; done
    for process in "${PIDS[@]}"; do wait "$process" 2>/dev/null || true; done
}
trap cleanup EXIT
export XDG_CONFIG_HOME="$TEST_ROOT/config" XDG_DATA_HOME="$TEST_ROOT/data"
export XDG_STATE_HOME="$TEST_ROOT/state" XDG_CACHE_HOME="$TEST_ROOT/cache"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export XDG_RUNTIME_DIR="$TEST_ROOT/runtime" HOME="$TEST_ROOT/home"
export QINGJIAN_SOCKET="$TEST_ROOT/server.sock" QINGJIAN_RESOURCES="$TEST_ROOT"
export QINGJIAN_DICT="$TEST_ROOT/dict.tsv"
unset IBUS_ADDRESS FCITX_DBUS_ADDRESS DBUS_STARTER_ADDRESS DBUS_STARTER_BUS_TYPE
export WAYLAND_DISPLAY='' XDG_SESSION_TYPE=x11 XDG_CURRENT_DESKTOP=QingjianTest
export GDK_BACKEND=x11 NO_AT_BRIDGE=1 GIO_USE_VFS=local
mkdir -p "$XDG_CONFIG_HOME/qingjian" "$XDG_DATA_HOME" "$XDG_STATE_HOME" \
    "$XDG_RUNTIME_DIR" "$HOME" "$TEST_ROOT/data/generated/schemes/wubi86"
chmod 700 "$XDG_RUNTIME_DIR"
printf '[general]\nscheme = "none"\ncode = "wubi86"\npage_size = 2\n' > "$XDG_CONFIG_HOME/qingjian/config.toml"
printf '你\tni\t100\n好\thao\t100\n' > "$QINGJIAN_DICT"
printf 'name = "测试五笔"\ncode_length = 4\nphrase_rule = "wubi86"\n' > "$TEST_ROOT/data/generated/schemes/wubi86/scheme.toml"
printf '一\tggll\t100\n开\tga\t10\n甲\taaaa\t10\n乙\taaaa\t5\n' > "$TEST_ROOT/data/generated/schemes/wubi86/table.tsv"
Xvfb -displayfd 3 -screen 0 1024x768x24 3>"$TEST_ROOT/display" >"$TEST_ROOT/xvfb.log" 2>&1 &
PIDS+=("$!")
for ((attempt = 0; attempt < 600; attempt++)); do
    [[ -s "$TEST_ROOT/display" ]] && break
    sleep 0.05
done
DISPLAY=":$(cat "$TEST_ROOT/display")"
export DISPLAY
dbus-update-activation-environment DISPLAY HOME WAYLAND_DISPLAY XDG_SESSION_TYPE XDG_CURRENT_DESKTOP \
    XDG_CONFIG_HOME XDG_DATA_HOME XDG_STATE_HOME XDG_CACHE_HOME XDG_RUNTIME_DIR GIO_USE_VFS
"$ROOT/target/debug/qingjian-linux-server" >"$TEST_ROOT/server.log" 2>&1 &
PIDS+=("$!")
for ((attempt = 0; attempt < 600; attempt++)); do
    [[ -S "$QINGJIAN_SOCKET" ]] && break
    sleep 0.05
done
[[ -S "$QINGJIAN_SOCKET" ]]
case "$FRONTEND" in
    fcitx4)
        FCITX4_ROOT=${FCITX4_ROOT:?Provide an extracted Fcitx4 runtime, without installing conflicting packages}
        mkdir -p "$XDG_CONFIG_HOME/fcitx/addon"
        for addon in "$FCITX4_ROOT"/usr/share/fcitx/addon/*.conf; do
            sed "s|^Library=|Library=$FCITX4_ROOT/usr/lib/$(gcc -dumpmachine)/fcitx/|" "$addon" > "$XDG_CONFIG_HOME/fcitx/addon/$(basename "$addon")"
        done
        sed "s|^Library=.*|Library=$ROOT/target/fcitx4-wubi/fcitx-qingjian.so|" \
            "$ROOT/apps/linux/fcitx4/data/addon/fcitx-qingjian.conf" > "$XDG_CONFIG_HOME/fcitx/addon/fcitx-qingjian.conf"
        printf '[Profile]\nEnabledIMList=fcitx-keyboard-us:True,qingjian:True\nIMName=qingjian\n' > "$XDG_CONFIG_HOME/fcitx/profile"
        export XDG_DATA_DIRS="$FCITX4_ROOT/usr/share:/usr/local/share:/usr/share"
        export FCITXDIR="$FCITX4_ROOT/usr"
        export FCITX4_REMOTE="$FCITX4_ROOT/usr/bin/fcitx-remote"
        "$FCITX4_ROOT/usr/bin/fcitx" -D --disable=all \
            --enable=fcitx-dbus,fcitx-x11,fcitx-xim,fcitx-ipc,fcitx-remote-module,fcitx-keyboard,fcitx-xkb,fcitx-qingjian,fcitx-classic-ui \
            >"$TEST_ROOT/frontend.log" 2>&1 &
        PIDS+=("$!")
        export GTK_IM_MODULE=xim XMODIFIERS=@im=fcitx
        ;;
    fcitx5)
        mkdir -p "$XDG_DATA_HOME/fcitx5/addon" "$XDG_DATA_HOME/fcitx5/inputmethod" "$XDG_CONFIG_HOME/fcitx5"
        cp "$ROOT/apps/linux/fcitx5/data/addon/qingjian.conf" "$XDG_DATA_HOME/fcitx5/addon/"
        cp "$ROOT/apps/linux/fcitx5/data/inputmethod/qingjian.conf" "$XDG_DATA_HOME/fcitx5/inputmethod/"
        printf '[Groups/0]\nName=Default\nDefault Layout=us\nDefaultIM=qingjian\n[Groups/0/Items/0]\nName=keyboard-us\n[Groups/0/Items/1]\nName=qingjian\n[GroupOrder]\n0=Default\n' > "$XDG_CONFIG_HOME/fcitx5/profile"
        FCITX_ADDON_DIRS="$ROOT/target/fcitx5-wubi:$(pkg-config --variable=libdir Fcitx5Core)/fcitx5"
        export FCITX_ADDON_DIRS
        fcitx5 -D --disable=wayland --verbose=key_trace=5 >"$TEST_ROOT/frontend.log" 2>&1 &
        PIDS+=("$!")
        export GTK_IM_MODULE=fcitx XMODIFIERS=@im=fcitx
        ;;
    ibus)
        mkdir -p "$TEST_ROOT/component"
        "$ROOT/target/debug/qingjian-linux-ibus" --ibus-xml "$ROOT/target/debug/qingjian-linux-ibus" > "$TEST_ROOT/component/qingjian.xml"
        export IBUS_COMPONENT_PATH="$TEST_ROOT/component" IBUS_ADDRESS="unix:path=$TEST_ROOT/bus"
        ibus-daemon -a "$IBUS_ADDRESS" -p disable -c disable -t refresh >"$TEST_ROOT/frontend.log" 2>&1 &
        PIDS+=("$!")
        for ((attempt = 0; attempt < 600; attempt++)); do
            [[ -S "$TEST_ROOT/bus" ]] && break
            sleep 0.05
        done
        if [[ ${RUN_IBUS_PROTOCOL:-1} == 1 ]]; then
            QINGJIAN_IBUS_WUBI_TEST=1 CARGO_BUILD_JOBS=3 timeout 90s cargo test --manifest-path "$ROOT/Cargo.toml" \
                -p qingjian-linux-ibus --locked --test ibus_engine wubi_commits -- --nocapture
        fi
        export GTK_IM_MODULE=ibus XMODIFIERS=@im=ibus
        ;;
    *) printf 'Unknown frontend: %s\n' "$FRONTEND" >&2; exit 2 ;;
esac
sleep 2
/usr/bin/python3 "$ROOT/apps/linux/tests/wubi/widget.py" "$FRONTEND"
