//! Fcitx4 前端不提供选区时，从 X11 PRIMARY 读取用户当前选中的文字。
#pragma once
#include <string>

namespace qingjian {
std::string primarySelection();
}
