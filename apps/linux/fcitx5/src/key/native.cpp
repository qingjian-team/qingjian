//! 无法事后回送按键的客户端复用同一 IPC 队列，在原始请求内返回消费结果。
#include "../qingjian.h"

namespace fcitx {
bool QingjianEngine::nativeKey(InputContext *context, const Key &key, bool release, int time) {
    const auto result = std::make_shared<std::optional<bool>>();
    const auto alive = alive_;
    const auto generation = shared_->generation;
    if (!nativeWaiting_)
        nativeDeadline_ = std::chrono::steady_clock::now() + std::chrono::milliseconds(200);
    const auto deadline = nativeDeadline_;
    ++nativeWaiting_;
    queueKey(context, key, release, time, result);
    while (*alive && generation == shared_->generation && !result->has_value() && busy_) {
        if (std::chrono::steady_clock::now() >= deadline || !async_->wait(deadline)) {
            disconnectAll("native-key-timeout");
            break;
        }
    }
    if (!*alive) return true;
    --nativeWaiting_;
    pump();
    return result->value_or(true);
}
}
