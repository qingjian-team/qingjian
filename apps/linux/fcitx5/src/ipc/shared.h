//! 插件唯一连接、观察器与安全上下文登记；析构通知延迟到下一事务。
#pragma once
#include "connection.h"
#include <fcitx/inputcontext.h>
#include <fcitx-utils/event.h>
#include <unordered_map>
#include <vector>
namespace qingjian {
class SharedConnection {
public:
    Connection connection;

    std::unique_ptr<fcitx::EventSourceIO> watcher;

    std::unique_ptr<fcitx::EventSource> deferred;

    uint64_t generation = 0;

    uint64_t nextSession = 1;

    bool clearing = false;

    std::unordered_map<uint64_t, fcitx::TrackableObjectReference<fcitx::InputContext>> contexts;

    std::vector<uint64_t> retired;

    void closeLater(uint64_t id) {
        if (!connection.connected()) return;
        retired.push_back(id);
        if (deferred) deferred->setEnabled(true);
    }
    void retire(uint64_t id) { contexts.erase(id); closeLater(id); }
    bool flush() {
        auto pending = std::move(retired);
        retired.clear();
        for (auto id : pending)
            if (!connection.send({{"CloseSession", {{"session", id}}}})) return false;
        return true;
    }
};
}
