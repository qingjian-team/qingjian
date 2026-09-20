//! 可控 Server 夹具，只模拟返回帧，记录插件发送的框架事实。
#pragma once
#include "wire.h"
#include <cstring>
#include <filesystem>
#include <thread>
#include <sys/un.h>
#include <unistd.h>
namespace qingjian::test {
struct Mock {
    std::string directory;

    int listener;

    std::thread thread;

    std::vector<Json> messages;

    explicit Mock(const std::string &mode) {
        char path[] = "/tmp/qingjian-plugin-XXXXXX";
        assert(mkdtemp(path));
        directory = path;
        std::string socketPath = directory + "/server.sock";
        setenv("QINGJIAN_SOCKET", socketPath.c_str(), 1);
        setenv("XDG_CONFIG_HOME", path, 1);
        setenv("XDG_DATA_HOME", path, 1);
        unsetenv("DBUS_SESSION_BUS_ADDRESS");
        listener = socket(AF_UNIX, SOCK_STREAM, 0);
        sockaddr_un address{}; address.sun_family = AF_UNIX;
        std::strcpy(address.sun_path, socketPath.c_str());
        assert(bind(listener, reinterpret_cast<sockaddr *>(&address), sizeof(address)) == 0);
        assert(listen(listener, 2) == 0);
        thread = std::thread([this, mode] { serve(mode); });
    }
    ~Mock() {
        join();
        close(listener);
        std::filesystem::remove_all(directory);
    }
    void join() { if (thread.joinable()) thread.join(); }
    void serve(const std::string &mode) {
        int fd = accept(listener, nullptr, nullptr);
        auto opened = readMessage(fd).at("OpenSession");
        const auto session = opened.at("session");
        assert(opened.at("protocol") == 6);
        writeMessage(fd, {{"Update", {{"session", session}, {"linux_ui", {{"version", mode == "mismatch" ? 2 : 3}}}}}});
        if (mode == "mismatch") { close(fd); return; }
        auto identity = readMessage(fd).at("LinuxHello");
        assert(identity.at("version") == 3);
        identity.erase("version"); identity.erase("session");
        identity["revision"] = 0;
        writeMessage(fd, {{"LinuxHello", {{"version", 3}, {"session", session}, {"preedit", mode == "inline" || mode == "window" ? mode : "both"}}}});
        auto frame = emptyFrame();
        char byte;
        while (recv(fd, &byte, 1, MSG_PEEK) > 0) {
            auto message = readMessage(fd);
            messages.push_back(message);
            if (message.contains("CloseSession")) break;
            if (message.contains("DisplayAcknowledged")) continue;
            const auto &event = message.at("LinuxEvent").at("event");
            Json commit = nullptr;
            std::string outcome = "Passthrough";
            if (event.contains("Key") && !event.at("Key").at("release").get<bool>() && event.at("Key").at("event").at("character") == "n") {
                frame = Json::parse(R"({"preedit":[{"text":"ni","kind":"Typed"}],"cursor":2,"candidates":{"items":[{"text":"","translation":null},{"text":"","translation":null},{"text":"你好","translation":{"senses":[{"text":"hello","fresh":true}]}}]},"highlight":2,"page":0,"page_count":2,"notice":null})");
                outcome = "Consumed";
            }
            if (event.contains("Candidate")) {
                assert(event.at("Candidate").at("index") == 2);
                assert(event.at("Candidate").at("identity") == identity);
                commit = "你好";
                frame = emptyFrame();
                outcome = "Consumed";
            }
            if (event.contains("Capabilities") || event == "Reset") frame = emptyFrame();
            if (event.contains("Deactivate")) {
                const auto &facts = event.at("Deactivate");
                if (!facts.at("capability_changed").get<bool>() && !(facts.at("focus_out").get<bool>() && facts.at("client_preedit").get<bool>()) && !frame.at("preedit").empty()) commit = "ni";
                frame = emptyFrame();
            }
            identity["revision"] = identity.at("revision").get<uint64_t>() + 1;
            writeMessage(fd, {{"KeyResult", {{"session", session}, {"identity", identity}, {"frame", frame}, {"commit", commit}, {"outcome", outcome}}}});
            if (mode == "failure" && event.contains("Key")) {
                messages.push_back(readMessage(fd)); // 默认面板反馈。
                break;
            }
        }
        close(fd);
    }
};
}
