//! 能力同步清理重入时，真实 Server 的学习与输入日志必须遵守最新隐私能力。
#include "qingjian.h"
#include "support/context.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
#include <cassert>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <thread>
#include <sys/wait.h>
#include <signal.h>
#include <unistd.h>
int main(int argc, char **argv) {
    assert(argc == 4);
    const std::string scenario = argv[3];
    char directory[] = "/tmp/qingjian-privacy-reentry-XXXXXX";
    assert(mkdtemp(directory));
    const auto userDirectory = std::filesystem::path(directory) / "qingjian";
    std::filesystem::create_directories(userDirectory);
    std::ofstream(userDirectory / "config.toml") << "[general]\ninput_log = true\nlearning = true\n";
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
        Context context(instance.inputContextManager());
        using Flag = fcitx::CapabilityFlag;
        using Flags = fcitx::CapabilityFlags;
        context.setCapabilityFlags(Flag::Preedit);
        context.focusIn();
        auto type = [&](const std::string &text, bool consumed = true) {
            for (char c : text) assert(engine.process(&context, fcitx::Key(static_cast<fcitx::KeySym>(c))) == consumed);
        };
        type("kaifa ");
        assert(context.committed == "开发");
        type("nihao"); // 旧普通组句也必须在能力边界被丢弃。
        bool armed = true;
        const bool blocked = scenario == "password-reset" || scenario == "disable-focus"
            || scenario == "superseded-password" || scenario == "superseded-disable";
        auto watcher = instance.watchEvent(fcitx::EventType::InputContextUpdatePreedit, fcitx::EventWatcherPhase::PreInputMethod, [&](fcitx::Event &) {
            if (!armed) return;
            armed = false;
            if (scenario == "sensitive-reset" || scenario == "password-reset") {
                fcitx::InputMethodEntry entry("qingjian", "qingjian", "zh_CN", "qingjian");
                fcitx::InputContextEvent reset(&context, fcitx::EventType::InputContextReset);
                engine.reset(entry, reset);
            } else if (scenario == "sensitive-focus" || scenario == "disable-focus") {
                context.focusOut();
                context.focusIn();
            } else if (scenario == "superseded-password") context.setCapabilityFlags(Flags(Flag::Preedit) | Flag::Sensitive | Flag::Password);
            else if (scenario == "superseded-disable") context.setCapabilityFlags(Flags(Flag::Preedit) | Flag::Sensitive | Flag::Disable);
            else if (scenario == "superseded-sensitive") context.setCapabilityFlags(Flags(Flag::Preedit) | Flag::Sensitive);
            else assert(scenario == "sensitive-key");
            // 在 clear 尚未返回时直接重入按键，不能沿用 Server 的旧普通状态。
            type("nihao ", !blocked);
        });
        context.setCapabilityFlags(Flags(Flag::Preedit) |
            (scenario == "password-reset" || scenario == "superseded-sensitive" ? Flag::Password :
             scenario == "disable-focus" ? Flag::Disable : Flag::Sensitive));
        assert(!armed);
        type("nihao ", !blocked); // 清理返回后缓存也必须与 Server 一致。
        assert(context.committed == (blocked ? "开发" : "开发你好你好"));
        context.setCapabilityFlags(Flag::Preedit);
        type("zhongguo ");
        assert(context.committed.ends_with("中国"));
    }
    kill(server, SIGTERM);
    int status = 0;
    waitpid(server, &status, 0);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 0);
    auto read = [](const std::filesystem::path &path) {
        std::ifstream input(path);
        return std::string(std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>());
    };
    const auto learning = read(userDirectory / "user.tsv");
    const auto log = read(userDirectory / "logs/input-log.jsonl");
    assert(learning.find("开发") != std::string::npos && learning.find("中国") != std::string::npos);
    assert(log.find("开发") != std::string::npos && log.find("中国") != std::string::npos);
    assert(learning.find("你好") == std::string::npos);
    assert(log.find("你好") == std::string::npos && log.find("nihao") == std::string::npos);
    std::filesystem::remove_all(directory);
}
