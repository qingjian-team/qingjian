//! 真 Server 响应绘制同步重入框架生命周期时，撤销旧提交并保留已消费按键。
#include "qingjian.h"
#include "support/context.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
#include <cassert>
#include <chrono>
#include <filesystem>
#include <stdexcept>
#include <thread>
#include <sys/wait.h>
#include <signal.h>
#include <unistd.h>
int main(int argc, char **argv) {
    assert(argc == 4);
    const std::string scenario = argv[3];
    char directory[] = "/tmp/qingjian-reentry-XXXXXX";
    assert(mkdtemp(directory));
    const std::string socketPath = std::string(directory) + "/server.sock";
    const std::string dictionary = std::string(argv[2]) + "/assets/sample/dict.tsv";
    setenv("QINGJIAN_SOCKET", socketPath.c_str(), 1);
    setenv("QINGJIAN_RESOURCES", directory, 1);
    setenv("QINGJIAN_DICT", dictionary.c_str(), 1);
    setenv("XDG_CONFIG_HOME", directory, 1);
    setenv("XDG_DATA_HOME", directory, 1);
    setenv("XDG_STATE_HOME", directory, 1);
    unsetenv("DBUS_SESSION_BUS_ADDRESS");
    const auto server = fork();
    assert(server >= 0);
    if (server == 0) { execl(argv[1], argv[1], nullptr); _exit(127); }
    for (int i = 0; i < 500 && !std::filesystem::exists(socketPath); ++i) std::this_thread::sleep_for(std::chrono::milliseconds(10));
    assert(std::filesystem::exists(socketPath));
    {
        char program[] = "qingjian-test"; char disable[] = "--disable=all";
        char *arguments[] = {program, disable, nullptr};
        fcitx::Instance instance(2, arguments);
        instance.initialize();
        fcitx::QingjianEngine engine(&instance.addonManager());
        auto owned = std::make_unique<Context>(instance.inputContextManager());
        auto *context = owned.get();
        context->setCapabilityFlags(fcitx::CapabilityFlag::Preedit);
        context->focusIn();
        for (char c : std::string("nihao")) assert(engine.process(context, fcitx::Key(static_cast<fcitx::KeySym>(c))));
        auto list = context->inputPanel().candidateList();
        assert(list && list->candidate(0).text().toString() == "你好");
        bool armed = true;
        const auto eventType = scenario == "ui-reset" ? fcitx::EventType::InputContextUpdateUI : fcitx::EventType::InputContextUpdatePreedit;
        auto watcher = instance.watchEvent(eventType, fcitx::EventWatcherPhase::PreInputMethod, [&](fcitx::Event &) {
            if (!armed || scenario == "destroy") return;
            armed = false;
            if (scenario == "password" || scenario == "click") context->setCapabilityFlags(fcitx::CapabilityFlag::Password);
            else if (scenario == "sensitive") context->setCapabilityFlags(fcitx::CapabilityFlag::Sensitive);
            else if (scenario == "disable") context->setCapabilityFlags(fcitx::CapabilityFlag::Disable);
            else if (scenario == "reset" || scenario == "ui-reset") {
                fcitx::InputMethodEntry entry("qingjian", "qingjian", "zh_CN", "qingjian");
                fcitx::InputContextEvent reset(context, fcitx::EventType::InputContextReset);
                engine.reset(entry, reset);
            } else if (scenario == "focus") {
                context->focusOut();
                context->focusIn(); // 最终 hasFocus 仍为 true，生命周期必须拒绝旧提交。
            } else if (scenario == "display-failure") throw std::runtime_error("测试显示失败");

            else assert(false);
        });
        // 从 frontend 回调结束处销毁；不能在 Fcitx 自己仍遍历事件时释放它的原始指针。
        if (scenario == "destroy") context->onPreedit = [&] { armed = false; owned.reset(); };
        if (scenario == "click") list->candidate(0).select(context);
        else assert(engine.process(context, fcitx::Key(FcitxKey_space)));
        assert(!armed);
        if (owned) {
            assert(context->committed == (scenario == "display-failure" ? "你好" : ""));
            list->candidate(0).select(context);
            assert(context->committed == (scenario == "display-failure" ? "你好" : ""));
        }
    }
    kill(server, SIGTERM);
    int status = 0;
    waitpid(server, &status, 0);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 0);
    std::filesystem::remove_all(directory);
}
