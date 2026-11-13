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
    if actual != "开一乙开开一":
        finish(AssertionError(repr(actual)))
        return False
    entry.set_text("")
    config = os.path.join(os.environ["XDG_CONFIG_HOME"], "qingjian", "config.toml")
    with open(config + ".tmp", "w", encoding="utf-8") as output:
        output.write(
            '[general]\nscheme = "pinyin"\ncode = "wubi86"\npage_size = 2\n'
            '[code.wubi86]\nmixed_auto_commit = true\nmixed_top = true\n'
        )
    os.replace(config + ".tmp", config)
    GLib.timeout_add(2000, mixed_step, 0)
    return False


MIXED_STEPS = [
    ("ga", "", None),
    (" ggl", "开", None),
    ("l", "开一", None),
    ("aaaa", "开一", None),
    ("ggll", "开一甲一", "Down"),
]


def mixed_step(index):
    def send():
        try:
            text, _, function_key = MIXED_STEPS[index]
            if function_key:
                run("xdotool", "key", function_key)
            run("xdotool", "type", "--clearmodifiers", "--delay", "100", text)
            GLib.timeout_add(500, check_mixed_step, index)
        except Exception as error:
            GLib.idle_add(finish, error)

    threading.Thread(target=send, daemon=True).start()
    return False


def check_mixed_step(index):
    actual = entry.get_text()
    expected = MIXED_STEPS[index][1]
    if actual != expected:
        finish(AssertionError(f"mixed step {index}: {actual!r} != {expected!r}"))
    elif index + 1 == len(MIXED_STEPS):
        finish(None)
    else:
        mixed_step(index + 1)
    return False


def finish(error):
    global failed
    failed = error is not None
    result = "FAIL " + str(error) if failed else "PASS 纯五笔组词、混输四码唯一上屏与第五键续输"
    print(f"GTK3 Wubi {sys.argv[1]}: {result}")
    Gtk.main_quit()


GLib.timeout_add(1000, type_input)
GLib.timeout_add_seconds(60, lambda: finish(TimeoutError("GTK3 test timed out")))
Gtk.main()
sys.exit(1 if failed else 0)
