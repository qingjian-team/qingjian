//! 无法事后回送按键的客户端复用同一 IPC 队列，在原始请求内返回消费结果。
#include "../qingjian.h"

namespace fcitx {
bool QingjianEngine::nativeKey(InputContext *context, const Key &key, bool release, int time) {
    const auto result = std::make_shared<std::optional<bool>>();
    const auto alive = alive_;
    ++nativeWaiting_;
    queueKey(context, key, release, time, result);
    while (*alive && !result->has_value() && busy_) {
        if (!async_->wait()) break;
    }
    if (!*alive) return true;
    --nativeWaiting_;
    pump();
    return result->value_or(true);
}
}
