//! 传输错误在有界时间内使整条连接失效，已发送事件不重放。
#include "qingjian.h"
#include "support/input.h"
#include "support/wire.h"
#include <fcitx/inputcontextmanager.h>
#include <chrono>
#include <cstring>
#include <filesystem>
#include <thread>
#include <sys/un.h>
#include <unistd.h>
int main(int argc, char **argv) {
    assert(argc == 2);
    const std::string failure = argv[1];
    char directory[] = "/tmp/qingjian-transport-XXXXXX"; assert(mkdtemp(directory));
    const std::string socketPath = std::string(directory) + "/server.sock";
    setenv("QINGJIAN_SOCKET", socketPath.c_str(), 1);
    setenv("XDG_CONFIG_HOME", directory, 1); setenv("XDG_DATA_HOME", directory, 1);
    int listener = socket(AF_UNIX, SOCK_STREAM, 0);
    sockaddr_un address{}; address.sun_family = AF_UNIX; std::strcpy(address.sun_path, socketPath.c_str());
    assert(bind(listener, reinterpret_cast<sockaddr *>(&address), sizeof(address)) == 0);
    assert(listen(listener, 2) == 0);
    int keys = 0;
    std::thread mock([&] {
        int connection = accept(listener, nullptr, nullptr);
        auto opened = readMessage(connection).at("OpenSession");
        const auto session = opened.at("session");
        assert(opened.at("protocol") == 6);
        writeMessage(connection, {{"Update", {{"session", session}, {"linux_ui", {{"version", failure == "old-version" ? 2 : 3}}}}}});
        if (failure != "old-version") {
            auto identity = readMessage(connection).at("LinuxHello");
            identity.erase("version"); identity.erase("session"); identity["revision"] = 1;
            writeMessage(connection, {{"LinuxHello", {{"session", session}, {"version", 3}, {"preedit", "both"}}}});
            for (const auto *name : {"Capabilities", "Focus"}) {
                const auto event = readMessage(connection).at("LinuxEvent").at("event");
                assert(event.contains(name));
                identity["revision"] = identity.at("revision").get<uint64_t>() + 1;
                writeMessage(connection, {{"KeyResult", {{"session", session}, {"identity", identity},
                    {"frame", emptyFrame()}, {"commit", nullptr}, {"outcome", "Passthrough"}}}});
            }
            assert(readMessage(connection).at("LinuxEvent").at("event").contains("Key")); ++keys;
            if (failure == "timeout") std::this_thread::sleep_for(std::chrono::milliseconds(350));
            else if (failure == "malformed") {
                const char bad[] = {1, 0, 0, 0, '{'};
                assert(send(connection, bad, sizeof(bad), MSG_NOSIGNAL) == sizeof(bad));
            } else writeMessage(connection, {{"KeyResult", {{"session", 999}, {"outcome", "Consumed"}, {"commit", "不可上屏"}, {"identity", identity}, {"frame", Json::object()}}}});
        }
        char byte; assert(recv(connection, &byte, 1, 0) == 0); // 失败后关闭，无键重放。
        close(connection);
    });
    char program[] = "qingjian-transport"; char disable[] = "--disable=all"; char *arguments[] = {program, disable, nullptr};
    {
        fcitx::Instance instance(2, arguments); instance.initialize();
        fcitx::QingjianEngine engine(&instance.addonManager());
        Input context(instance.inputContextManager());
        auto begin = std::chrono::steady_clock::now();
        assert(!engine.process(&context, fcitx::Key(FcitxKey_n)));
        assert(std::chrono::steady_clock::now() - begin < std::chrono::seconds(1));
        assert(context.committed.empty() && context.inputPanel().empty());
        mock.join();
        assert(keys == (failure == "old-version" ? 0 : 1));
    }
    close(listener); std::filesystem::remove_all(directory);
}
