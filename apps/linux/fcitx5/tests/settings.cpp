//! 配置入口向 Fcitx5 配置工具提供唯一的外部设置命令。
#include "qingjian.h"
#include <cassert>
#include <cstdlib>
#include <fstream>
#include <fcitx-config/rawconfig.h>

int main() {
    char program[] = "qingjian-settings-test";
    char disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    fcitx::InputMethodEntry entry("qingjian", "qingjian", "zh_CN", "qingjian");
    const auto *config = engine.getConfigForInputMethod(entry);
    assert(config);
    fcitx::RawConfig description;
    config->dumpDescription(description);
    const auto type = description.get(config->typeName());
    assert(type && type->subItems().size() == 1);
    const auto option = type->get("Settings");
    assert(option && option->get("Type")->value() == "External");
    assert(option->get("External")->value() == "qingjian-settings");
    char temporary[] = "/tmp/qingjian-settings-XXXXXX";
    const auto *created = mkdtemp(temporary);
    assert(created);
    const std::filesystem::path directory(created);
    const auto prefix = directory / "prefix with spaces";
    const auto executable = prefix / "bin/qingjian-settings";
    std::filesystem::create_directories(executable.parent_path());
    std::ofstream(executable) << "#!/bin/sh\n";
    const auto library = prefix / "lib/x86_64-linux-gnu/fcitx5/qingjian.so";
    assert(qingjian::settingsCommandForLibrary(library) == "qingjian-settings");
    std::filesystem::permissions(executable, std::filesystem::perms::owner_exec,
                                 std::filesystem::perm_options::add);
    assert(qingjian::settingsCommandForLibrary(library) == "\"" + executable.string() + "\"");
    const auto quotedPrefix = directory / "prefix\"quoted";
    std::filesystem::rename(prefix, quotedPrefix);
    assert(qingjian::settingsCommandForLibrary(quotedPrefix / "lib/fcitx5/qingjian.so") ==
           "\"" + directory.string() + "/prefix\"\"\"quoted/bin/qingjian-settings\"");
    std::filesystem::remove_all(directory);
}
