//! Fcitx5 薄插件：框架事件转发、默认面板与上屏适配。
#pragma once
#include "session.h"
#include "settings/config.h"
#include "ipc/async.h"
#include "key/pending.h"
#include <deque>
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
    const Configuration *getConfig() const override { return &settings_; }
    void keyEvent(const InputMethodEntry &, KeyEvent &) override;
    void reset(const InputMethodEntry &, InputContextEvent &) override;
    void deactivate(const InputMethodEntry &, InputContextEvent &) override;
    bool process(InputContext *context, const Key &key, bool release = false);
private:
    void clear(InputContext *context);
    void disconnect(InputContext *context);
    void disconnectAll(const char *reason);
    /// 仅在面板有未完成输入时记录生命周期原因，不记录输入内容。
    void logLifecycle(InputContext *context, const char *reason);
    bool connect(InputContext *context);
    bool syncPrivacy(InputContext *context);
    bool exchange(InputContext *context, const nlohmann::json &event, bool display = true,
                  const nlohmann::json *received = nullptr);
    void enqueue(qingjian::PendingEvent event, bool priority = false);
    void queueEvent(InputContext *context, const nlohmann::json &event, bool display = true,
                    bool priority = false, bool wire = false);
    void pump();
    bool notice(InputContext *context, const nlohmann::json &message);
    void queueKey(InputContext *context, const Key &key, bool release, int time);
    void render(InputContext *context, const nlohmann::json &frame);
    /// 组句期间定时向 Server 发 Poll 取异步结果（本地整句模型重排）；帧为空就停。
    void watch(InputContext *context, bool composing);
    void stopPolling();
    void poll(const nlohmann::json *received = nullptr);

    /// 生产按键路径的唯一后台连接；兼容接口在首次框架按键之前仍可同步调用。
    std::unique_ptr<qingjian::AsyncConnection> async_;

    /// 仅事件线程访问，最多保留 1024 项未发送的框架事实。
    std::deque<qingjian::PendingEvent> pending_;

    /// 在途事务含主线程应用响应，避免回调重入时插队发送下一键。
    bool busy_ = false;

    /// 下一次后台任务开始前关闭旧连接，禁止恢复旧会话或重放请求。
    bool reconnect_ = false;

    Instance *instance_;

    qingjian::SettingsConfig settings_;

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

    /// 中 / 英切换提示：按键处理里不能直接弹，挪到事件循环下一拍（见 exchange 里的注释）。
    std::unique_ptr<EventSource> osdDefer_;

    TrackableObjectReference<InputContext> osdContext_;

    std::string osdText_;
};
}
