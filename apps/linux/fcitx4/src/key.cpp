//! 映射 Fcitx4 键值；字符由 keysym 转换为 UTF-8。
#include "key.h"
#include <fcitx-config/hotkey.h>
#include <fcitx-utils/utf8.h>
#include <string>

namespace qingjian {
nlohmann::json mapKey(FcitxKeySym key, unsigned int state) {
    uint32_t code = key;
    switch (key) {
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
        if (key >= FcitxKey_KP_0 && key <= FcitxKey_KP_9) code = 0x60 + key - FcitxKey_KP_0;
        break;
    }
    const std::string shiftedDigits = ")!@#$%^&*(";
    auto shifted = shiftedDigits.find(static_cast<char>(key));
    if (key < 128 && (state & FcitxKeyState_Shift) && shifted != std::string::npos)
        code = 0x30 + shifted;
    nlohmann::json character = nullptr;
    uint32_t unicode = FcitxKeySymToUnicode(key);
    if (unicode >= 0x20 && unicode != 0x7f) {
        char utf8[8]{};
        int bytes = fcitx_ucs4_to_utf8(unicode, utf8);
        if (bytes > 0) character = std::string(utf8, bytes);
    }
    return {{"virtual_key", code}, {"character", character}, {"modifiers", {
        {"ctrl", (state & FcitxKeyState_Ctrl) != 0},
        {"shift", (state & FcitxKeyState_Shift) != 0 || key == FcitxKey_ISO_Left_Tab},
        {"alt", (state & FcitxKeyState_Alt) != 0},
        {"win", (state & (FcitxKeyState_Super | FcitxKeyState_Super2 | FcitxKeyState_Hyper)) != 0},
        {"caps", (state & FcitxKeyState_CapsLock) != 0}, {"english_mode", false}}}};
}
}
