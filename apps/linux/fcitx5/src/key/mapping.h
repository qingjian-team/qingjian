//! Fcitx KeySym 与共享协议键码之间的映射。
#pragma once
#include <fcitx-utils/key.h>
#include <nlohmann/json.hpp>
namespace qingjian {
nlohmann::json mapKey(const fcitx::Key &key);
}
