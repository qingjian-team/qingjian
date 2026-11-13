//! 按插件安装位置查找配套设置程序，避免依赖桌面会话的 PATH。
#pragma once
#include <filesystem>
#include <string>

namespace qingjian {
std::string settingsCommandForLibrary(const std::filesystem::path &library);
std::string settingsCommand();
}
