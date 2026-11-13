//! 有界 Unix socket 传输；任何错误都关闭连接，下一次按键重新握手。
#pragma once
#include <nlohmann/json.hpp>
#include <atomic>
#include <chrono>
namespace qingjian {
class Connection {
public:
    explicit Connection(std::chrono::milliseconds timeout = std::chrono::milliseconds(200),
                        const std::atomic<bool> *cancelled = nullptr)
        : timeout_(timeout), cancelled_(cancelled) {}
    Connection(const Connection &) = delete;
    Connection &operator=(const Connection &) = delete;
    ~Connection();
    bool open();
    bool send(const nlohmann::json &message, nlohmann::json *response = nullptr);
    bool connected() const { return fd_ >= 0; }
    /// 只供事件循环观察断线；收发仍统一通过 send。
    int fd() const { return fd_; }
    void close();
private:
    /// 当前连接；-1 表示未连接。
    int fd_ = -1;

    /// 同步兼容接口与后台收发使用不同的有界等待时间。
    std::chrono::milliseconds timeout_;

    /// 后台任务退出时中断等待，避免卸载插件等待整个超时周期。
    const std::atomic<bool> *cancelled_;
};
}
