//! 真实 Rust Server 隔离夹具；词库、配置、用户数据只写临时目录。
#pragma once
#include <cassert>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <thread>
#include <signal.h>
#include <sys/wait.h>
#include <sys/prctl.h>
#include <unistd.h>
class Server {
public:
    std::filesystem::path directory;

    pid_t pid = -1;

    explicit Server(const std::string &general = "") {
        char pattern[] = "/tmp/qingjian-real-XXXXXX";
        auto *created = mkdtemp(pattern); assert(created);
        directory = created;
        setenv("QINGJIAN_SOCKET", (directory / "server.sock").c_str(), 1);
        setenv("QINGJIAN_RESOURCES", directory.c_str(), 1);
        setenv("QINGJIAN_DICT", (directory / "dict.tsv").c_str(), 1);
        setenv("XDG_CONFIG_HOME", (directory / "config").c_str(), 1);
        setenv("XDG_DATA_HOME", (directory / "data").c_str(), 1);
        setenv("XDG_STATE_HOME", (directory / "state").c_str(), 1);
        std::filesystem::create_directories(directory / "config/qingjian");
        std::ofstream(directory / "dict.tsv") << "你\tni\t100\n泥\tni\t90\n拟\tni\t80\n你好\tni hao\t100\n好\thao\t80\n开发\tkai fa\t90\n";
        std::ofstream(directory / "config/qingjian/config.toml") << "[general]\ninput_log = true\npage_size = 2\n" << general << "\n[dictionaries]\ndomains = []\n";
        std::filesystem::create_directories(directory / "data/generated");
        std::ofstream(directory / "data/generated/english.tsv") << "hello\thello\t100\nhelp\thelp\t90\nheld\theld\t80\n";
        start();
    }
    ~Server() { stop(); std::filesystem::remove_all(directory); }
    void stop() {
        if (pid > 0) { kill(pid, SIGTERM); int status; waitpid(pid, &status, 0); pid = -1; }
    }
    void start() {
        pid = fork(); assert(pid >= 0);
        if (pid == 0) { prctl(PR_SET_PDEATHSIG, SIGKILL); execl(QINGJIAN_SERVER_EXECUTABLE, QINGJIAN_SERVER_EXECUTABLE, nullptr); _exit(127); }
        for (int i = 0; i < 500; ++i) {
            if (std::filesystem::exists(directory / "server.sock")) return;
            int status; assert(waitpid(pid, &status, WNOHANG) == 0);
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        assert(false);
    }
    size_t sockets() const {
        size_t count = 0;
        for (const auto &entry : std::filesystem::directory_iterator("/proc/" + std::to_string(pid) + "/fd")) {
            std::error_code error;
            auto target = std::filesystem::read_symlink(entry.path(), error).string();
            if (!error && target.rfind("socket:", 0) == 0) ++count;
        }
        return count;
    }
};
