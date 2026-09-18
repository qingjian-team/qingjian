//! Fcitx 上下文只保存连接与展示事实，输入状态保存在 Server。
#pragma once
#include "ipc/connection.h"
#include <fcitx/inputcontextproperty.h>
#include <fcitx-utils/event.h>
namespace qingjian {
struct Session final : fcitx::InputContextProperty {
    Connection connection;

    std::unique_ptr<fcitx::EventSourceIO> socketWatcher;

    uint64_t id = 1;

    bool opened = false;

    bool focused = false;

    /// 最近提交给客户端的预编辑；框架失焦提交事实依赖此字段。
    bool clientPreedit = false;

    uint64_t revision = 0;

    uint64_t generation = 0;

    /// 框架边界代次；与纯显示失败分开，使重入后的旧提交可被撤销。
    uint64_t lifecycle = 0;

    nlohmann::json displayIdentity;

    /// 仅缓存 Server 已确认的能力；尚未成功发送不能视为同步。
    nlohmann::json capabilities;

    std::string preeditMode = "both";
};
}
