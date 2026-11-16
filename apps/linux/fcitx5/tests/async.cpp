//! 真实 Server 暂停期间排队按键；事件循环、焦点隔离与析构不能被 IPC 阻塞。
#include "qingjian.h"
#include "support/server.h"
#include "support/input.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputmethodentry.h>
#include <cassert>
#include <chrono>

int main(int argc, char **argv) {
    assert(argc == 2);
    const std::string mode = argv[1];
    if (mode == "idle-shutdown") {
        fcitx::EventLoop loop;
        for (size_t iteration = 0; iteration < 500; ++iteration) {
            qingjian::AsyncConnection connection(loop);
            std::this_thread::yield();
        }
        return 0;
    }
    Server server("scheme = \"none\"\ncode = \"wubi86\"\n",
                  "不错\tgiqa\t100\n错\tqa\t100\n开\tga\t100\n甲\taaaa\t10\n乙\tbbbb\t10\n");
    char program[] = "qingjian-async", disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    auto engine = std::make_unique<fcitx::QingjianEngine>(&instance.addonManager());
    auto input = std::make_unique<Input>(instance.inputContextManager());
    input->focusIn();
    const fcitx::InputMethodEntry entry("qingjian", "Qingjian", "zh", "qingjian");
    auto emit = [&](fcitx::Key key, bool release = false) {
        fcitx::KeyEvent event(input.get(), key, release, 123);
        const auto before = std::chrono::steady_clock::now();
        engine->keyEvent(entry, event);
        assert(event.filtered());
        assert(std::chrono::steady_clock::now() - before < std::chrono::milliseconds(100));
    };
    if (mode == "reentry") {
        input->commitHook = [&] {
            for (auto character : std::string("ga "))
                emit(fcitx::Key(static_cast<fcitx::KeySym>(character)));
        };
    }
    assert(kill(server.pid, SIGSTOP) == 0);
    for (auto character : std::string("giqa")) {
        emit(fcitx::Key(static_cast<fcitx::KeySym>(character)));
        emit(fcitx::Key(static_cast<fcitx::KeySym>(character)), true);
    }
    emit(fcitx::Key(FcitxKey_F1));
    emit(fcitx::Key(FcitxKey_F1), true);
    int heartbeats = 0;
    auto heartbeat = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 10000, 1000,
        [&](fcitx::EventSourceTime *source, uint64_t) {
            ++heartbeats;
            source->setNextInterval(10000); source->setOneShot(); return true;
        });
    auto boundary = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 50000, 1000,
        [&](fcitx::EventSourceTime *, uint64_t) {
            if (mode == "focus") input->focusOut();
            if (mode == "focus-return" || mode == "privacy-return") {
                if (mode == "focus-return") {
                    input->focusOut();
                    fcitx::FocusOutEvent event(input.get());
                    engine->deactivate(entry, event);
                    input->focusIn();
                } else {
                    input->setCapabilityFlags(fcitx::CapabilityFlag::Password);
                    input->setCapabilityFlags(fcitx::CapabilityFlag::Preedit);
                }
                for (auto character : std::string("ga "))
                    emit(fcitx::Key(static_cast<fcitx::KeySym>(character)));
            }
            if (mode == "password") {
                input->setCapabilityFlags(fcitx::CapabilityFlag::Password);
                fcitx::KeyEvent secret(input.get(), fcitx::Key(FcitxKey_h));
                engine->keyEvent(entry, secret);
                assert(!secret.filtered());
            }
            if (mode == "destroy") input.reset();
            if (mode == "reset") {
                fcitx::InputContextEvent event(input.get(), fcitx::EventType::InputContextReset);
                engine->reset(entry, event);
            }
            if (mode == "shutdown") {
                const auto before = std::chrono::steady_clock::now();
                engine.reset();
                assert(std::chrono::steady_clock::now() - before < std::chrono::milliseconds(150));
            }
            return false;
        });
    auto resume = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 600000, 1000,
        [&](fcitx::EventSourceTime *, uint64_t) {
            assert(heartbeats > 20);
            assert(kill(server.pid, SIGCONT) == 0);
            return false;
        });
    auto finish = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 1200000, 1000,
        [&](fcitx::EventSourceTime *, uint64_t) {
            assert(heartbeats > 20);
            if (mode == "stall" || mode == "reentry") {
                assert(input->committed == (mode == "stall" ? "不错" : "不错开"));
                assert(input->forwarded.size() >= 2);
                const auto &press = input->forwarded[input->forwarded.size() - 2];
                const auto &release = input->forwarded.back();
                assert(press.first.sym() == FcitxKey_F1 && !press.second);
                assert(release.first.sym() == FcitxKey_F1 && release.second);
            } else if (mode == "focus-return" || mode == "privacy-return") {
                assert(input->committed == "开");
            } else if (input) {
                assert(input->committed.empty());
                assert(input->forwarded.empty());
                assert(input->inputPanel().empty());
            }
            instance.eventLoop().exit(); return false;
        });
    instance.eventLoop().exec();
    engine.reset();
    kill(server.pid, SIGCONT);
    kill(server.pid, SIGTERM);
    waitpid(server.pid, nullptr, 0);
}
