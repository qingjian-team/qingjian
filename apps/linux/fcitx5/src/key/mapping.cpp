//! 字符交给 JSON 库转义；功能键转换成共享协议的 VK 值。
#include "mapping.h"
namespace qingjian {
nlohmann::json mapKey(const fcitx::Key &key) {
    uint32_t code = key.sym();
    switch (key.sym()) {
    case FcitxKey_BackSpace: code = 0x08; break;
    case FcitxKey_Tab: case FcitxKey_ISO_Left_Tab: code = 0x09; break;
    case FcitxKey_Return: case FcitxKey_KP_Enter: code = 0x0d; break;
    case FcitxKey_Escape: code = 0x1b; break;
    case FcitxKey_Page_Up: case FcitxKey_KP_Page_Up: code = 0x21; break;
    case FcitxKey_Page_Down: case FcitxKey_KP_Page_Down: code = 0x22; break;
    case FcitxKey_End: case FcitxKey_KP_End: code = 0x23; break;
    case FcitxKey_Home: case FcitxKey_KP_Home: code = 0x24; break;
    case FcitxKey_Left: case FcitxKey_KP_Left: code = 0x25; break;
    case FcitxKey_Up: case FcitxKey_KP_Up: code = 0x26; break;
    case FcitxKey_Right: case FcitxKey_KP_Right: code = 0x27; break;
    case FcitxKey_Down: case FcitxKey_KP_Down: code = 0x28; break;
    case FcitxKey_Delete: case FcitxKey_KP_Delete: code = 0x2e; break;
    case FcitxKey_KP_Insert: code = 0x2d; break;
    case FcitxKey_KP_Begin: code = 0x0c; break;
    case FcitxKey_Shift_L: case FcitxKey_Shift_R: code = 0x10; break;
    case FcitxKey_KP_Multiply: code = 0x6a; break;
    case FcitxKey_KP_Add: code = 0x6b; break;
    case FcitxKey_KP_Separator: code = 0x6c; break;
    case FcitxKey_KP_Subtract: code = 0x6d; break;
    case FcitxKey_KP_Decimal: code = 0x6e; break;
    case FcitxKey_KP_Divide: code = 0x6f; break;
    default:
        if (key.sym() >= FcitxKey_KP_0 && key.sym() <= FcitxKey_KP_9) code = 0x60 + key.sym() - FcitxKey_KP_0;
        break;
    }
    // Shift+数字的译词/删除快捷键按物理数字行识别，字符仍保留 !@# 等。
    const std::string shiftedDigits = ")!@#$%^&*(";
    auto shifted = shiftedDigits.find(static_cast<char>(key.sym()));
    if (key.sym() < 128 && key.states().test(fcitx::KeyState::Shift) && shifted != std::string::npos) code = 0x30 + shifted;
    auto unicode = fcitx::Key::keySymToUnicode(key.sym());
    nlohmann::json character = nullptr;
    if (unicode >= 0x20 && unicode != 0x7f) character = fcitx::Key::keySymToUTF8(key.sym());
    auto states = key.states();
    return {{"virtual_key", code}, {"character", character}, {"modifiers", {
        {"ctrl", states.test(fcitx::KeyState::Ctrl)}, {"shift", states.test(fcitx::KeyState::Shift) || key.sym() == FcitxKey_ISO_Left_Tab},
        {"alt", states.test(fcitx::KeyState::Alt)}, {"win", states.test(fcitx::KeyState::Super) || states.test(fcitx::KeyState::Hyper)},
        {"caps", states.test(fcitx::KeyState::CapsLock)}, {"english_mode", false}}}};
}
}
