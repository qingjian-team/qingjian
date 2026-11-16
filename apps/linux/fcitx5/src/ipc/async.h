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
private:
    void run();

    /// 工作线程只读取取消标记，不访问 Engine 或 InputContext。
    std::atomic<bool> stopping_{false};

    fcitx::EventDispatcher dispatcher_;

    std::mutex mutex_;

    std::condition_variable ready_;

    /// 单个待执行事务；取出后只有事件线程收到完成通知才提交下一项。
    std::optional<std::pair<Work, Done>> job_;

    std::thread worker_;

    /// 已投递回调在主线程检查此标记，防止 Engine 析构后继续执行。
    std::shared_ptr<bool> alive_ = std::make_shared<bool>(true);
};
}
