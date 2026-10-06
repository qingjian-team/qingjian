//! 单个 Fcitx4 输入上下文的 Server 会话身份和显示事实。
#pragma once
#include <fcitx/frontend.h>
#include <nlohmann/json.hpp>
#include <cstdint>
#include <string>

namespace qingjian {
struct Session {
    uint64_t id = 0;

    uint64_t generation = 0;

    uint64_t lifecycle = 0;

    FcitxInputContext *context = nullptr;

    bool opened = false;

    bool focused = false;

    bool clientPreedit = false;

    bool composing = false;

    std::string preeditMode = "both";

    nlohmann::json identity;

    nlohmann::json capabilities;

    std::string selectionText;

    unsigned int selectionCursor = 0;

    unsigned int selectionAnchor = 0;
};
}
