# //! 暂停私有测试 Server 后，快速输入的 giqa 仍应完整转换；不操作桌面服务。
import os
import signal
import subprocess
import sys
import threading
import time

import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk


window = Gtk.Window(title="Qingjian isolated timeout reproduction")
entry = Gtk.Entry()
entry.set_property("im-module", os.environ["GTK_IM_MODULE"])
window.add(entry)
window.show_all()
entry.grab_focus()
server = int(os.environ["QINGJIAN_WUBI_TEST_SERVER_PID"])
failed = False


def run(*arguments):
    subprocess.run(arguments, check=True, timeout=10)


def reproduce():
    try:
        identifier = subprocess.check_output(
            ["xdotool", "search", "--name", "Qingjian isolated timeout reproduction"], text=True
        ).splitlines()[0]
        run("xdotool", "windowfocus", "--sync", identifier)
        run("fcitx5-remote", "-s", "qingjian")
        run("fcitx5-remote", "-o")
        time.sleep(1)
        run("xdotool", "key", "Escape")
        time.sleep(0.5)
        run("xdotool", "type", "--clearmodifiers", "--delay", "100", "giqa")
        GLib.timeout_add(1000, check_baseline)
    except Exception as error:
        GLib.idle_add(finish, error)


def check_baseline():
    actual = entry.get_text()
    if actual != "不错":
        finish(AssertionError(f"baseline: {actual!r}"))
    else:
        print("Without IPC stall: giqa → 不错")
        entry.set_text("")
        threading.Thread(target=stall, daemon=True).start()
    return False


def stall():
    try:
        os.kill(server, signal.SIGSTOP)
        try:
            run("xdotool", "type", "--clearmodifiers", "--delay", "300", "gi")
            time.sleep(0.5)
        finally:
            os.kill(server, signal.SIGCONT)
        time.sleep(1)
        run("xdotool", "type", "--clearmodifiers", "--delay", "100", "qa ")
        GLib.timeout_add(1000, check)
    except Exception as error:
        GLib.idle_add(finish, error)


def check():
    actual = entry.get_text()
    print(f"Expected input: giqa + Space → 不错 + Space; actual after IPC stall: {actual!r}")
    finish(None if actual == "不错 " else AssertionError(repr(actual)))
    return False


def finish(error):
    global failed
    failed = error is not None
    print("IPC stall regression PASS" if not failed else f"IPC stall regression FAIL: {error}")
    Gtk.main_quit()
    return False


def start():
    threading.Thread(target=reproduce, daemon=True).start()
    return False


GLib.timeout_add(1000, start)
GLib.timeout_add_seconds(30, lambda: finish(TimeoutError("reproduction timed out")))
try:
    Gtk.main()
finally:
    try:
        os.kill(server, signal.SIGCONT)
    except ProcessLookupError:
        pass
sys.exit(1 if failed else 0)
