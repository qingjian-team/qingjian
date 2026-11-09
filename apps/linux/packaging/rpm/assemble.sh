#!/usr/bin/env bash
# 在 Fedora 42 容器里跑（build.sh 调，仓库挂在 /src）：编 fcitx5 插件摆进 $stage/root，再用 qingjian.spec 出两个 rpm。
# Server、IBus 前端与词库 build.sh 已经摆好在 $stage/root。
set -euo pipefail
stage=/src/target/linux-pkg/rpm
root="$stage/root"
libdir=$(rpm --eval '%{_libdir}')

build="$stage/fcitx5-build"
cmake -S /src/apps/linux/fcitx5 -B "$build" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF >/dev/null
cmake --build "$build" --parallel >/dev/null
install -Dm755 "$build/qingjian.so" "$root$libdir/fcitx5/qingjian.so"
strip --strip-unneeded "$root$libdir/fcitx5/qingjian.so" "$root/usr/bin/qingjian-linux-server" "$root/usr/bin/qingjian-linux-ibus"
install -Dm644 /src/apps/linux/fcitx5/data/addon/qingjian.conf "$root/usr/share/fcitx5/addon/qingjian.conf"
install -Dm644 /src/apps/linux/fcitx5/data/inputmethod/qingjian.conf "$root/usr/share/fcitx5/inputmethod/qingjian.conf"
# 宿主机 umask 是 002 时拷出来的是 664 / 775
find "$root" -type d -exec chmod 755 {} +
find "$root" -type f ! -path '*/usr/bin/*' ! -name '*.so' -exec chmod 644 {} +

rpmbuild -bb /src/apps/linux/packaging/rpm/qingjian.spec \
  --define "_topdir $stage/rpmbuild" --define "_rpmdir $stage/rpms" \
  --define "qj_root $root" --define "qj_version ${RPM_VERSION:?}" --define "qj_release ${RPM_RELEASE:?}" >/dev/null
ls "$stage"/rpms/*/*.rpm
