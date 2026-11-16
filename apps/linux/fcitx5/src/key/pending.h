//! 排队的框架事件：按生命周期作废旧请求，不保存业务候选或自行决定选词。
#pragma once
#include <fcitx/inputcontext.h>
#include <nlohmann/json.hpp>
#include <optional>

namespace qingjian {
struct PendingEvent {
    fcitx::TrackableObjectReference<fcitx::InputContext> context;

    /// 捕获框架生命周期与连接代次，边界之后不发送、不应用旧事件。
    uint64_t lifecycle = 0;

    uint64_t generation = 0;

    nlohmann::json event;

    /// 仅 Server 明确回复 Passthrough 后，按原始按下 / 抬起事实转发。
    std::optional<fcitx::Key> key;

    bool release = false;

    int time = 0;

    bool focused = false;

    bool display = true;

    /// 顶层单向通知，不等待 KeyResult；无上下文时仅允许关闭会话。
    bool wire = false;

    /// 空闲轮询使用顶层 Poll，不能被后续按键抢先应用。
    bool poll = false;
};
}
