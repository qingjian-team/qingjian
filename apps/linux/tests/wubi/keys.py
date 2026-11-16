# //! GTK3 / GTK4 真实文本框对比原生按键与五笔激活后的文本、光标和选区。
import os
import signal
import subprocess
import threading
import time

import gi

VERSION = os.environ.get("QINGJIAN_GTK_VERSION", "4.0")
gi.require_version("Gtk", VERSION)
from gi.repository import GLib, Gtk

window = Gtk.Window(title="Qingjian isolated key test")
view = Gtk.TextView()
window.set_default_size(600, 250)
if VERSION == "3.0":
    window.add(view)
    window.show_all()
else:
    window.set_child(view)
    window.present()
view.grab_focus()
loop = GLib.MainLoop()
errors = []


def run(*arguments):
    return subprocess.check_output(arguments, text=True, timeout=10).strip()


def on_ui(function):
    done = threading.Event()
    result = []

    def invoke():
        try:
            result.append(function())
        except Exception as error:
            errors.append(error)
        finally:
            done.set()
        return False

    GLib.idle_add(invoke)
    if not done.wait(10):
        raise TimeoutError("GTK callback timed out")
    if errors:
        raise errors[0]
    return result[0]


def reset(text, position):
    buffer = view.get_buffer()
    buffer.set_text(text)
    buffer.place_cursor(buffer.get_iter_at_offset(position))


def snapshot():
    buffer = view.get_buffer()
    return (
        buffer.get_text(buffer.get_start_iter(), buffer.get_end_iter(), True),
        buffer.get_iter_at_mark(buffer.get_insert()).get_offset(),
        tuple(iterator.get_offset() for iterator in buffer.get_selection_bounds()),
    )


CASES = [
    ("digits", "", 0, ["1", "2", "3", "0"]),
    ("keypad", "", 0, ["KP_1", "KP_2", "KP_0"]),
    ("space", "AB", 1, ["space"]),
    ("return", "AB", 1, ["Return"]),
    ("keypad-enter", "AB", 1, ["KP_Enter"]),
    ("tab", "AB", 1, ["Tab"]),
    ("backspace", "ABCD", 2, ["BackSpace"]),
    ("delete", "ABCD", 2, ["Delete"]),
    ("left", "ABCD", 2, ["Left"]),
    ("right", "ABCD", 2, ["Right"]),
    ("up", "ABCD\nEFGH\nIJKL", 7, ["Up"]),
    ("down", "ABCD\nEFGH\nIJKL", 7, ["Down"]),
    ("home", "ABCD\nEFGH\nIJKL", 7, ["Home"]),
    ("end", "ABCD\nEFGH\nIJKL", 7, ["End"]),
    ("page-up", "\n".join(["ABCD"] * 30), 127, ["Page_Up"]),
    ("page-down", "\n".join(["ABCD"] * 30), 7, ["Page_Down"]),
    ("ctrl-home", "ABCD\nEFGH\nIJKL", 7, ["ctrl+Home"]),
    ("ctrl-end", "ABCD\nEFGH\nIJKL", 7, ["ctrl+End"]),
    ("ctrl-left", "AB CD EF", 5, ["ctrl+Left"]),
    ("ctrl-right", "AB CD EF", 3, ["ctrl+Right"]),
    ("ctrl-backspace", "AB CD EF", 5, ["ctrl+BackSpace"]),
    ("ctrl-delete", "AB CD EF", 3, ["ctrl+Delete"]),
    ("select-all", "ABCD", 2, ["ctrl+a"]),
    ("shift-left", "ABCD", 2, ["shift+Left"]),
    ("shift-right", "ABCD", 2, ["shift+Right"]),
    ("cut", "ABCD", 2, ["ctrl+a", "ctrl+x"]),
    ("paste", "", 0, ["ctrl+v"]),
    ("slash", "", 0, ["slash"]),
    ("minus", "", 0, ["minus"]),
    ("equal", "", 0, ["equal"]),
]


def test():
    try:
        identifier = run("xdotool", "search", "--name", "^Qingjian isolated key test$").splitlines()[0]
        run("xdotool", "windowfocus", "--sync", identifier)
        time.sleep(1)
        run("fcitx5-remote", "-s", "qingjian")
        for label, text, position, keys in CASES:
            results = []
            for active in (False, True):
                run("fcitx5-remote", "-o" if active else "-c")
                time.sleep(0.1)
                on_ui(lambda: reset(text, position))
                time.sleep(0.1)
                if active and run("fcitx5-remote", "-n") != "qingjian":
                    raise AssertionError("Qingjian is not active")
                run("xdotool", "key", "--clearmodifiers", "--delay", "30", *keys)
                time.sleep(0.3)
                results.append(on_ui(snapshot))
            if label in ("page-up", "page-down"):
                direction = -1 if label == "page-up" else 1
                valid = all(actual[0] == text and (actual[1] - position) * direction > 0
                            and not actual[2] for actual in results)
            else:
                valid = results[0] == results[1]
            if not valid:
                raise AssertionError(f"{label}: native={results[0]!r}, qingjian={results[1]!r}")
            print(f"PASS GTK{VERSION} {label}", flush=True)
        for keys, expected in [
            (["a", "a", "a", "a", "2"], "乙"),
            (["g", "a", "space"], "开"),
            (["g", "a", "BackSpace", "a", "space"], "开"),
            (["g", "a", "Escape"], ""),
            (["comma", "period", "shift+1"], "，。！"),
        ]:
            on_ui(lambda: reset("", 0))
            time.sleep(0.1)
            run("xdotool", "key", "--clearmodifiers", "--delay", "10", *keys)
            time.sleep(0.3)
            actual = on_ui(snapshot)[0]
            if actual != expected:
                raise AssertionError(f"{keys!r}: expected={expected!r}, actual={actual!r}")
            print(f"PASS GTK{VERSION} composition {keys!r}", flush=True)
        for stalled in (False, True):
            on_ui(lambda: reset("", 0))
            time.sleep(0.1)
            server = int(os.environ["QINGJIAN_WUBI_TEST_SERVER_PID"])
            resume = threading.Timer(0.6, os.kill, (server, signal.SIGCONT))
            try:
                if stalled:
                    os.kill(server, signal.SIGSTOP)
                    resume.start()
                run("xdotool", "type", "--clearmodifiers", "--delay", "1", "giqa ")
                if stalled:
                    resume.join()
            finally:
                if stalled:
                    os.kill(server, signal.SIGCONT)
                    resume.join()
            time.sleep(0.5)
            actual = on_ui(snapshot)[0]
            if actual != "不错 ":
                raise AssertionError(f"fast input stalled={stalled}: {actual!r}")
            print(f"PASS GTK{VERSION} fast input stalled={stalled}", flush=True)
    except Exception as error:
        errors.append(error)
    finally:
        GLib.idle_add(loop.quit)


threading.Thread(target=test, daemon=True).start()
loop.run()
if errors:
    raise errors[0]
