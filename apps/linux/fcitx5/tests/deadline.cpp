//! 原始请求只等一个短截止时间；排队和重入不续期，超时后不应用晚到响应。
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
    Server server("scheme = \"none\"\ncode = \"wubi86\"\n", "开\tga\t100\n一\tggll\t100\n");
    char program[] = "qingjian-native-timeout", disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    auto engine = std::make_unique<fcitx::QingjianEngine>(&instance.addonManager());
    Input input(instance.inputContextManager());
    input.backend = "dbus";
    input.focusIn();
    Input asynchronous(instance.inputContextManager());
    asynchronous.focusIn();
    const fcitx::InputMethodEntry entry("qingjian", "Qingjian", "zh", "qingjian");
    auto emit = [&](Input &context, fcitx::KeySym symbol) {
        fcitx::KeyEvent event(&context, fcitx::Key(symbol));
        engine->keyEvent(entry, event);
        return event.filtered();
    };
    assert(emit(input, FcitxKey_g));
    assert(emit(input, FcitxKey_a));
    const bool queued = mode == "queued" || mode == "reentry";
    if (queued) {
        asynchronous.commitHook = [&] {
            assert(kill(server.pid, SIGSTOP) == 0);
            if (mode == "reentry") assert(emit(input, FcitxKey_space));
        };
        for (auto symbol : {FcitxKey_g, FcitxKey_a, FcitxKey_space})
            assert(emit(asynchronous, symbol));
    }
    assert(kill(server.pid, SIGSTOP) == 0);
    std::thread resume([&] {
        if (queued) {
            std::this_thread::sleep_for(std::chrono::milliseconds(120));
            assert(kill(server.pid, SIGCONT) == 0);
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(700));
        assert(kill(server.pid, SIGCONT) == 0);
    });
    const auto before = std::chrono::steady_clock::now();
    const bool filtered = emit(input, FcitxKey_space);
    const auto elapsed = std::chrono::steady_clock::now() - before;
    std::chrono::steady_clock::duration shutdown{};
    if (mode == "shutdown") {
        const auto start = std::chrono::steady_clock::now();
        engine.reset();
        shutdown = std::chrono::steady_clock::now() - start;
    }
    resume.join();
    assert(filtered);
    assert(elapsed < std::chrono::milliseconds(280));
    assert(shutdown < std::chrono::milliseconds(100));
    assert(input.committed.empty());
    assert(input.preedit().empty());
    bool heartbeat = false;
    auto finish = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC,
        fcitx::now(CLOCK_MONOTONIC) + 100000, 1000,
        [&](fcitx::EventSourceTime *, uint64_t) {
            heartbeat = true;
            assert(input.committed.empty());
            assert(input.forwarded.empty());
            assert(asynchronous.forwarded.empty());
            if (engine) {
                assert(!emit(input, FcitxKey_1));
                for (auto symbol : {FcitxKey_g, FcitxKey_a, FcitxKey_space})
                    assert(emit(input, symbol));
                assert(input.committed == "开");
            }
            instance.eventLoop().exit();
            return false;
        });
    instance.eventLoop().exec();
    assert(heartbeat);
}
