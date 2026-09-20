//! Fcitx 上下文只保存会话身份与展示事实，输入状态保存在 Server。
#pragma once
#include "ipc/shared.h"
#include <fcitx/inputcontextproperty.h>
#include <fcitx-utils/event.h>
namespace qingjian {
struct Session final : fcitx::InputContextProperty {
    Session(std::shared_ptr<SharedConnection> shared, uint64_t number)
        : id(number), owner(shared) {}
    ~Session() override {
        if (auto shared = owner.lock()) {
            if (opened) shared->retire(id);
            else shared->contexts.erase(id);
        }
    }

    /// 插件生命周期内递增，永不复用。
    uint64_t id;

    /// 析构只撤销登记并排队关闭，不阻塞或访问已析构 Engine。
    std::weak_ptr<SharedConnection> owner;

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
