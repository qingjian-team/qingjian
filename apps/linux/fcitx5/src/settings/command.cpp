//! 设置程序与插件共享安装前缀；系统多架构 lib 目录同样向上查找 bin。
#include "command.h"
#include <dlfcn.h>
#include <unistd.h>

namespace qingjian {
std::string settingsCommandForLibrary(const std::filesystem::path &library) {
    for (auto directory = library.parent_path(); !directory.empty() && directory != directory.root_path();
         directory = directory.parent_path()) {
        const auto executable = directory / "bin/qingjian-settings";
        if (access(executable.c_str(), X_OK) != 0) continue;
        std::string command = "\"";
        for (const auto character : executable.string()) {
            command += character == '"' ? "\"\"\"" : std::string(1, character);
        }
        return command + "\"";
    }
    return "qingjian-settings";
}

std::string settingsCommand() {
    static const char location = 0;
    Dl_info info{};
    if (dladdr(&location, &info) && info.dli_fname) {
        return settingsCommandForLibrary(info.dli_fname);
    }
    return "qingjian-settings";
}
}
