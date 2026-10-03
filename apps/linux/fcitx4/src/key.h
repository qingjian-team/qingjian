//! Fcitx4 按键转换为 Server 共用协议。
#pragma once
#include <fcitx-utils/keysym.h>
#include <nlohmann/json.hpp>

namespace qingjian {
nlohmann::json mapKey(FcitxKeySym key, unsigned int state);
}
