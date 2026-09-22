//! Fcitx5 薄插件：框架事件转发、默认面板与上屏适配。
#pragma once
#include "session.h"
#include <fcitx/addonfactory.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/instance.h>
namespace fcitx {
class QingjianEngine final : public InputMethodEngineV2 {
public:
    explicit QingjianEngine(AddonManager *manager);
    ~QingjianEngine() override;
    void keyEvent(const InputMethodEntry &, KeyEvent &) override;
    void reset(const InputMethodEntry &, InputContextEvent &) override;
    void deactivate(const InputMethodEntry &, InputContextEvent &) override;
    bool process(InputContext *context, const Key &key, bool release = false);
private:
    void clear(InputContext *context);
    void disconnect(InputContext *context);
    void disconnectAll();
    bool connect(InputContext *context);
    bool syncPrivacy(InputContext *context);
    bool exchange(InputContext *context, const nlohmann::json &event, bool display = true);
    void render(InputContext *context, const nlohmann::json &frame);
    /// 组句期间定时向 Server 发 Poll 取异步结果（本地整句模型重排）；帧为空就停。
    void watch(InputContext *context, bool composing);
    void stopPolling();
    void poll();

    Instance *instance_;

    std::shared_ptr<qingjian::SharedConnection> shared_;

    std::shared_ptr<bool> alive_ = std::make_shared<bool>(true);

    FactoryFor<qingjian::Session> sessions_;

    std::unique_ptr<HandlerTableEntry<EventHandler>> capabilityWatcher_;

    std::unique_ptr<HandlerTableEntry<EventHandler>> focusWatcher_;

    std::unique_ptr<HandlerTableEntry<EventHandler>> keyboardWatcher_;

    /// 轮询定时器：建一次反复用，没在组句时禁用。
    std::unique_ptr<EventSourceTime> poller_;

    /// 正在轮询的上下文：只有聚焦且在组句的那一个。
    TrackableObjectReference<InputContext> polled_;
};
}
