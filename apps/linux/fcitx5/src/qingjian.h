//! Fcitx5 薄插件：框架事件转发、默认面板与上屏适配。
#pragma once
#include "session.h"
#include <fcitx/addonfactory.h>
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

    Instance *instance_;

    std::shared_ptr<qingjian::SharedConnection> shared_;

    std::shared_ptr<bool> alive_ = std::make_shared<bool>(true);

    FactoryFor<qingjian::Session> sessions_;

    std::unique_ptr<HandlerTableEntry<EventHandler>> capabilityWatcher_;

    std::unique_ptr<HandlerTableEntry<EventHandler>> focusWatcher_;

    std::unique_ptr<HandlerTableEntry<EventHandler>> keyboardWatcher_;
};
}
