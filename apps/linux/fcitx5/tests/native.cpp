//! 不支持事后按键回送的 D-Bus 客户端必须在原始请求内获得消费结果。
#include "qingjian.h"
#include "support/input.h"
#include "support/server.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputmethodentry.h>
#include <cassert>
#include <chrono>

int main(int argc, char **argv) {
    assert(argc == 2);
    const std::string mode = argv[1];
    Server server("scheme = \"none\"\ncode = \"wubi86\"\n",
                  "不错\tgiqa\t100\n开\tga\t100\n甲\taaaa\t10\n乙\taaaa\t5\n");
    char program[] = "qingjian-native", disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    Input input(instance.inputContextManager());
    input.backend = "dbus";
    input.focusIn();
    const fcitx::InputMethodEntry entry("qingjian", "Qingjian", "zh", "qingjian");
    auto emit = [&](Input &context, const fcitx::Key &key, bool release = false) {
        fcitx::KeyEvent event(&context, key, release, 123);
        engine.keyEvent(entry, event);
        return event.filtered();
    };
    auto type = [&](const std::string &text) {
        for (auto character : text)
            assert(emit(input, fcitx::Key(static_cast<fcitx::KeySym>(character))));
    };
    if (mode == "passthrough") {
        for (const auto *description : {"1", "0", "KP_1", "space", "Return", "KP_Enter", "Tab",
                "BackSpace", "Delete", "Left", "Right", "Up", "Down", "Home", "End", "Page_Up",
                "Page_Down", "Control+a", "Control+c", "Control+x", "Control+v",
                "Control+z", "Control+s", "Control+Home", "Shift+Left", "slash", "minus", "equal", "F1"}) {
            const fcitx::Key key(description);
            assert(key.sym() != FcitxKey_None);
            assert(!emit(input, key));
            emit(input, key, true);
        }
        assert(input.committed.empty());
    } else if (mode == "composition") {
        type("aaaa2");
        assert(input.committed == "乙");
        type("ga ");
        assert(input.committed == "乙开");
        assert(!emit(input, fcitx::Key(FcitxKey_1)));
        type("ga");
        assert(emit(input, fcitx::Key(FcitxKey_BackSpace)));
        assert(emit(input, fcitx::Key(FcitxKey_Escape)));
        assert(input.preedit().empty());
    } else if (mode == "queued" || mode == "async-reentry") {
        Input asynchronous(instance.inputContextManager());
        asynchronous.focusIn();
        if (mode == "async-reentry") asynchronous.commitHook = [&] { type("ga "); };
        for (auto character : std::string("ga "))
            assert(emit(asynchronous, fcitx::Key(static_cast<fcitx::KeySym>(character))));
        assert(!emit(input, fcitx::Key(FcitxKey_Left)));
        assert(asynchronous.committed == "开");
        assert(input.committed == (mode == "async-reentry" ? "开" : ""));
    } else if (mode == "reentry") {
        input.commitHook = [&] { type("ga "); };
        type("giqa");
        assert(input.committed == "不错开");
    } else if (mode == "focus" || mode == "privacy") {
        input.inputPanel().setClientPreedit(fcitx::Text("old"));
        input.updatePreedit();
        bool reached = false;
        input.preeditHook = [&] {
            reached = true;
            if (mode == "focus") input.focusOut();
            else input.setCapabilityFlags(fcitx::CapabilityFlag::Password);
        };
        assert(emit(input, fcitx::Key(FcitxKey_1)));
        assert(reached);
        assert(input.committed.empty());
    } else if (mode == "disconnect") {
        type("ga ");
        server.stop();
        assert(emit(input, fcitx::Key(FcitxKey_1)));
        assert(input.committed == "开");
    } else if (mode == "stall") {
        assert(kill(server.pid, SIGSTOP) == 0);
        std::thread resume([&] {
            std::this_thread::sleep_for(std::chrono::milliseconds(350));
            assert(kill(server.pid, SIGCONT) == 0);
        });
        type("giqa");
        resume.join();
        assert(input.committed == "不错");
    }
    assert(input.forwarded.empty());
}
