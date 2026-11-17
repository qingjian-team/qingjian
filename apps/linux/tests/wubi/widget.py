# //! 真实 GTK3 文本框的五笔上屏断言；自动化线程不阻塞 GTK 主循环。
import os
import subprocess
import sys
import threading
import time

import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk


window = Gtk.Window(title="Qingjian isolated Wubi test")
entry = Gtk.Entry()
entry.set_property("im-module", os.environ["GTK_IM_MODULE"])
window.add(entry)
window.set_default_size(500, 100)
window.show_all()
entry.grab_focus()
failed = False


def run(*arguments):
    subprocess.run(arguments, check=True, timeout=10)


def type_input():
    threading.Thread(target=focus_and_activate, daemon=True).start()
    return False


def focus_and_activate():
    try:
        identifier = subprocess.check_output(
            ["xdotool", "search", "--name", "Qingjian isolated Wubi test"], text=True
        ).splitlines()[0]
        run("xdotool", "windowfocus", "--sync", identifier)
        run("xdotool", "mousemove", "--window", identifier, "100", "50", "click", "1")
        time.sleep(1)
        if sys.argv[1] == "fcitx5":
            run("fcitx5-remote", "-s", "qingjian")
            run("fcitx5-remote", "-o")
        elif sys.argv[1] == "fcitx4":
            run(os.environ["FCITX4_REMOTE"], "-s", "qingjian")
            run(os.environ["FCITX4_REMOTE"], "-o")
        else:
            run("ibus", "engine", "qingjian")
        GLib.timeout_add(1500, send_keys)
    except Exception as error:
        GLib.idle_add(finish, error)


def send_keys():
    threading.Thread(target=send_keys_background, daemon=True).start()
    return False


def send_keys_background():
    try:
        run("xdotool", "key", "Escape")
        time.sleep(1)
        run("xdotool", "type", "--clearmodifiers", "--delay", "100", "ga ggllaaaa")
        run("xdotool", "key", "Down")
        run("xdotool", "type", "--clearmodifiers", "--delay", "100", "ga ")
        run("xdotool", "type", "--clearmodifiers", "--delay", "100", "gagg")
        GLib.timeout_add(1000, check_text)
    except Exception as error:
        GLib.idle_add(finish, error)


def check_text():
    actual = entry.get_text()
    finish(None if actual == "开一乙开开一" else AssertionError(repr(actual)))
    return False


def finish(error):
    global failed
    failed = error is not None
    print(f"GTK3 Wubi {sys.argv[1]}: {'FAIL ' + str(error) if failed else 'PASS 开一乙开开一（首次自动组词）'}")
    Gtk.main_quit()


GLib.timeout_add(1000, type_input)
GLib.timeout_add_seconds(60, lambda: finish(TimeoutError("GTK3 test timed out")))
Gtk.main()
sys.exit(1 if failed else 0)
