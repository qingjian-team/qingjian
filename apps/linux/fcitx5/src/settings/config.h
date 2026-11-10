//! 让 Fcitx5 配置工具直接启动青简设置，不重复实现配置界面。
#pragma once
#include "command.h"
#include <fcitx-config/configuration.h>

namespace qingjian {
FCITX_CONFIGURATION(SettingsConfig,
    fcitx::ExternalOption settings{this, "Settings", "青简", settingsCommand()};
);
}
