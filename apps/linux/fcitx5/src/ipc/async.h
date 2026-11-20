//! 单个后台 IPC 工作线程；只传 JSON，Fcitx 上下文及回调始终留在事件线程。
#pragma once
#include "connection.h"
#include <fcitx-utils/eventdispatcher.h>
#include <condition_variable>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <thread>

namespace qingjian {
class AsyncConnection {
public:
    using Reply = std::optional<nlohmann::json>;
    using Work = std::function<Reply(Connection &)>;
    using Done = std::function<void(Reply)>;

    explicit AsyncConnection(fcitx::EventLoop &loop);
    ~AsyncConnection();
    void submit(Work work, Done done);
    /// 在原始请求的统一截止时间内领取完成结果，不运行嵌套事件循环。
    bool wait(std::chrono::steady_clock::time_point deadline);
    void cancel();
private:
    void dispatch();
    void run();

    /// 工作线程只读取取消标记，不访问 Engine 或 InputContext。
    std::atomic<bool> stopping_{false};

    /// 中断在途收发；只有领取完成结果并提交下一项事务时才复位。
    std::atomic<bool> cancelled_{false};

    fcitx::EventDispatcher dispatcher_;

    std::mutex mutex_;

    std::condition_variable ready_;

    /// 单个待执行事务；取出后只有事件线程收到完成通知才提交下一项。
    std::optional<std::pair<Work, Done>> job_;

    /// 事件循环与同步兼容路径共同领取的单次完成结果。
    std::optional<std::pair<Done, Reply>> completed_;

    std::thread worker_;

    /// 已投递回调在主线程检查此标记，防止 Engine 析构后继续执行。
    std::shared_ptr<bool> alive_ = std::make_shared<bool>(true);
};
}
