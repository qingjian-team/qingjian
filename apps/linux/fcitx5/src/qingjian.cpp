//! 默认候选面板；连接失效时清除预编辑、放行输入，不重放提交。
#include "qingjian.h"
#include "candidate/list.h"
#include "candidate/word.h"
#include "key/mapping.h"
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
#include <fcitx/userinterfacemanager.h>
#include <iomanip>
#include <sstream>
#include <stdexcept>
#include <sys/socket.h>
#include <cerrno>

namespace fcitx {
namespace {
/// 组句期间多久问一次 Server：与 Windows DLL 的轮询间隔相同，本地整句模型的结果晚一拍到。
constexpr uint64_t POLL_INTERVAL_US = 80000;
/// 定时器精度：sd-event 把 0 当缺省的 250 ms，允许晚到四分之一秒，轮询就没了节拍。
constexpr uint64_t POLL_ACCURACY_US = 1000;
std::string contextIdentity(const InputContext *context) {
    std::ostringstream out;
    out << std::hex << std::setfill('0');
    for (auto byte : context->uuid()) out << std::setw(2) << static_cast<unsigned>(byte);
    return out.str();
}
}
QingjianEngine::QingjianEngine(AddonManager *manager)
    : instance_(manager->instance()), shared_(std::make_shared<qingjian::SharedConnection>()),
      sessions_([this](InputContext &context) {
          const auto id = shared_->nextSession++;
          shared_->contexts.emplace(id, context.watch());
          return new qingjian::Session(shared_, id);
      }) {
    shared_->deferred = instance_->eventLoop().addDeferEvent([this](EventSource *event) {
        event->setEnabled(false);
        if (shared_->connection.connected() && !shared_->flush()) disconnectAll();
        return true;
    });
    shared_->deferred->setEnabled(false);
    instance_->inputContextManager().registerProperty("qingjian-session", &sessions_);
    capabilityWatcher_ = instance_->watchEvent(EventType::InputContextCapabilityChanged, EventWatcherPhase::PreInputMethod, [this](Event &event) {
        auto *context = static_cast<InputContextEvent &>(event).inputContext();
        ++context->propertyFor(&sessions_)->lifecycle;
        if (context->propertyFor(&sessions_)->opened) syncPrivacy(context);
    });
    focusWatcher_ = instance_->watchEvent(EventType::InputContextFocusOut, EventWatcherPhase::PreInputMethod, [this](Event &event) {
        auto *context = static_cast<InputContextEvent &>(event).inputContext();
        auto *session = context->propertyFor(&sessions_);
        ++session->lifecycle;
        session->focused = false;
        if (session->opened) exchange(context, {{"Focus", {{"focused", false}}}}, false);
    });
    keyboardWatcher_ = instance_->watchEvent(EventType::VirtualKeyboardVisibilityChanged, EventWatcherPhase::PostInputMethod, [this](Event &) {
        if (!instance_->userInterfaceManager().isVirtualKeyboardVisible()) return;
        const auto contexts = shared_->contexts;
        for (const auto &[id, watched] : contexts) {
            (void)id;
            auto *context = watched.get();
            if (!context) continue;
            auto *session = context->propertyFor(&sessions_);
            if (session->opened && session->displayIdentity.is_object() && !shared_->connection.send({{"DisplayAcknowledged", {
                {"session", session->id}, {"identity", session->displayIdentity}, {"senses", nlohmann::json::array()}}}})) disconnectAll();
        }
    });
}
QingjianEngine::~QingjianEngine() {
    *alive_ = false;
    poller_.reset(); polled_.unwatch();
    capabilityWatcher_.reset(); focusWatcher_.reset(); keyboardWatcher_.reset();
    shared_->watcher.reset(); shared_->deferred.reset(); shared_->connection.close();
    const auto contexts = shared_->contexts;
    shared_->contexts.clear();
    for (const auto &[id, watched] : contexts) {
        (void)id;
        if (auto *context = watched.get()) {
            context->propertyFor(&sessions_)->opened = false;
            context->inputPanel().reset();
        }
    }
    sessions_.unregister();
}
void QingjianEngine::clear(InputContext *context) {
    if (polled_.get() == context) stopPolling();
    const auto watched = context->watch();
    auto *session = context->propertyFor(&sessions_);
    const auto revision = ++session->revision;
    session->displayIdentity = nullptr;
    session->clientPreedit = false;
    context->inputPanel().reset();
    context->updatePreedit();
    if (watched.get() && session->revision == revision)
        context->updateUserInterface(UserInterfaceComponent::InputPanel);
}
void QingjianEngine::disconnectAll() {
    if (shared_->clearing) return;
    shared_->clearing = true;
    shared_->watcher.reset();
    shared_->connection.close();
    stopPolling();
    shared_->retired.clear();
    const auto contexts = shared_->contexts;
    // 先让全部会话失效，再调用可重入的 UI；清理期间禁止重新连接。
    for (const auto &[id, watched] : contexts) {
        (void)id;
        if (auto *context = watched.get()) {
            auto *session = context->propertyFor(&sessions_);
            session->opened = false;
            session->focused = false;
            session->capabilities = nullptr;
            ++session->revision;
        }
    }
    for (const auto &[id, watched] : contexts)
        if (auto *context = watched.get(); context && shared_->contexts.contains(id)) clear(context);
    shared_->clearing = false;
}
void QingjianEngine::disconnect(InputContext *context) {
    auto *session = context->propertyFor(&sessions_);
    if (session->opened) {
        shared_->retire(session->id);
        session->id = shared_->nextSession++;
        shared_->contexts.emplace(session->id, context->watch());
    }
    session->opened = false;
    session->focused = false;
    session->capabilities = nullptr;
    clear(context);
}
bool QingjianEngine::connect(InputContext *context) {
    auto *session = context->propertyFor(&sessions_);
    const auto watched = context->watch();
    if (shared_->clearing) return false;
    if (session->opened) {
        if (session->focused != context->hasFocus()) {
            session->focused = context->hasFocus();
            exchange(context, {{"Focus", {{"focused", session->focused}}}}, false);
        }
        return watched.get() && session->opened;
    }
    try {
        if (!shared_->connection.connected()) {
            if (!shared_->connection.open()) return false;
            ++shared_->generation;
            shared_->retired.clear();
            shared_->watcher = instance_->eventLoop().addIOEvent(shared_->connection.fd(), IOEventFlag::In,
                [this](EventSourceIO *, int fd, IOEventFlags) {
                    char byte;
                    auto count = recv(fd, &byte, 1, MSG_PEEK | MSG_DONTWAIT);
                    if (count >= 0 || (errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR)) disconnectAll();
                    return true;
                });
        }
        if (!shared_->flush()) throw std::runtime_error("close exchange");
        session->generation = shared_->generation;
        nlohmann::json response;
        if (!shared_->connection.send({{"OpenSession", {{"session", session->id}, {"app", context->program()}, {"protocol", 6}}}}, &response)
            || response.at("Update").at("session") != session->id
            || response.at("Update").at("linux_ui").at("version") != 3)
            throw std::runtime_error("protocol mismatch");
        if (!shared_->connection.send({{"LinuxHello", {{"version", 3}, {"session", session->id}, {"generation", session->generation}, {"context", contextIdentity(context)}}}}, &response)
            || response.at("LinuxHello").at("version") != 3 || response.at("LinuxHello").at("session") != session->id) throw std::runtime_error("linux handshake");
        session->preeditMode = response.at("LinuxHello").at("preedit").get<std::string>();
        if (session->preeditMode != "both" && session->preeditMode != "window" && session->preeditMode != "inline")
            throw std::runtime_error("preedit mode");
        session->opened = true;
        if (!syncPrivacy(context)) return false;
        session->focused = context->hasFocus();
        exchange(context, {{"Focus", {{"focused", session->focused}}}}, false);
        return watched.get() && session->opened;
    } catch (const std::exception &) { disconnectAll(); return false; }
}
bool QingjianEngine::syncPrivacy(InputContext *context) {
    auto *session = context->propertyFor(&sessions_);
    const auto caps = context->capabilityFlags();
    nlohmann::json facts = {{"sensitive", caps.test(CapabilityFlag::Sensitive)},
        {"password", caps.test(CapabilityFlag::Password)}, {"disabled", caps.test(CapabilityFlag::Disable)}};
    if (session->capabilities != facts) {
        const auto watched = context->watch();
        const auto generation = session->generation;
        // 能力先由 Server 确认，再触发可能同步重入的 UI 清理。
        // clear 中的 Reset / Focus / 按键已处于新隐私状态；更新能力由内层覆盖，
        // 外层清理返回后不再写缓存，避免把更近的能力覆盖回旧值。
        exchange(context, {{"Capabilities", facts}}, false);
        if (!watched.get() || !session->opened || session->generation != generation) return false;
        session->capabilities = facts;
        session->clientPreedit = false;
        clear(context);
        if (!watched.get() || !session->opened) return false;
    }
    // 能力清理可重入；以回调后的最新能力决定是否关闭此会话。
    const auto currentCaps = context->capabilityFlags();
    if (currentCaps.test(CapabilityFlag::Password) || currentCaps.test(CapabilityFlag::Disable)) {
        disconnect(context);
        return false;
    }
    return session->opened;
}
bool QingjianEngine::exchange(InputContext *context, const nlohmann::json &event, bool display) {
    auto *session = context->propertyFor(&sessions_);
    const auto watched = context->watch();
    const auto generation = session->generation;
    const auto lifecycle = session->lifecycle;
    const auto capabilities = context->capabilityFlags();
    const bool focused = context->hasFocus();
    bool consumed = false;
    try {
        nlohmann::json response;
        if (!session->opened || !shared_->connection.send({{"LinuxEvent", {{"session", session->id}, {"event", event}}}}, &response))
            throw std::runtime_error("event exchange");
        if (response.contains("Ignored") && response.at("Ignored").at("session") == session->id) return true;
        const auto &result = response.at("KeyResult");
        const auto &identity = result.at("identity");
        const auto outcome = result.at("outcome").get<std::string>();
        const auto &commit = result.at("commit");
        if (result.at("session") != session->id || (outcome != "Consumed" && outcome != "Passthrough")
            || (!commit.is_null() && !commit.is_string()) || identity.at("generation") != session->generation
            || identity.at("context") != contextIdentity(context) || !identity.at("revision").is_number_unsigned()
            || (!session->displayIdentity.is_null() && identity.at("revision").get<uint64_t>() <= session->displayIdentity.at("revision").get<uint64_t>()))
            throw std::runtime_error("response identity");
        consumed = outcome == "Consumed";
        session->displayIdentity = identity;
        // 显示 API 可同步重入能力 / 焦点 / Reset 事件。提交前再核对生命周期，
        // 撤销旧上下文结果时仍保留 Consumed，防止选词键再次透传。
        bool validDisplayFailure = false;
        if (display) {
            try { render(context, result.at("frame")); }
            catch (const std::exception &) {
                if (watched.get()) {
                    validDisplayFailure = session->displayIdentity == identity;
                    disconnectAll();
                }
            }
        }
        if (watched.get() && session->generation == generation && session->lifecycle == lifecycle
            && context->hasFocus() == focused && context->capabilityFlags() == capabilities
            && (session->displayIdentity == identity || validDisplayFailure) && commit.is_string())
            context->commitString(commit.get<std::string>());
        return outcome == "Consumed";
    } catch (const std::exception &) { disconnectAll(); return consumed; }
}
void QingjianEngine::reset(const InputMethodEntry &, InputContextEvent &event) {
    auto *context = event.inputContext();
    const auto watched = context->watch();
    ++context->propertyFor(&sessions_)->lifecycle;
    if (context->propertyFor(&sessions_)->opened) exchange(context, "Reset", false);
    if (!watched.get()) return;
    context->propertyFor(&sessions_)->clientPreedit = false;
    clear(context);
}
void QingjianEngine::deactivate(const InputMethodEntry &, InputContextEvent &event) {
    auto *context = event.inputContext();
    auto *session = context->propertyFor(&sessions_);
    const auto watched = context->watch();
    ++session->lifecycle;
    auto *switched = dynamic_cast<InputContextSwitchInputMethodEvent *>(&event);
    // 转发框架提前发出的切换原因；能力此刻可能仍是旧值。
    if (session->opened) {
        bool clientPreedit = session->clientPreedit;
        syncPrivacy(context);
        if (!watched.get()) return;
        if (session->opened) exchange(context, {{"Deactivate", {
            {"focus_out", event.type() == EventType::InputContextFocusOut}, {"client_preedit", clientPreedit},
            {"capability_changed", switched && switched->reason() == InputMethodSwitchedReason::CapabilityChanged}}}}, false);
    }
    if (!watched.get()) return;
    session->focused = false;
    session->clientPreedit = false;
    clear(context);
}
void QingjianEngine::keyEvent(const InputMethodEntry &, KeyEvent &event) {
    if (event.inputContext() && process(event.inputContext(), event.rawKey(), event.isRelease())) event.filterAndAccept();
}
bool QingjianEngine::process(InputContext *context, const Key &key, bool release) {
    const auto watched = context->watch();
    const auto caps = context->capabilityFlags();
    if (caps.test(CapabilityFlag::Password) || caps.test(CapabilityFlag::Disable)) {
        if (context->propertyFor(&sessions_)->opened) syncPrivacy(context);
        return false;
    }
    if (!connect(context) || !watched.get() || !syncPrivacy(context) || !watched.get()) return false;
    return exchange(context, {{"Key", {{"event", qingjian::mapKey(key)}, {"release", release}}}});
}
void QingjianEngine::render(InputContext *context, const nlohmann::json &frame) {
    auto *session = context->propertyFor(&sessions_);
    const auto identity = session->displayIdentity;
    const auto revision = ++session->revision;
    Text preedit;
    for (const auto &segment : frame.at("preedit")) preedit.append(segment.at("text").get<std::string>(), TextFormatFlag::Underline);
    auto raw = preedit.toString();
    size_t cursor = frame.at("cursor").get<size_t>();
    size_t bytes = 0;
    for (size_t chars = 0; bytes < raw.size() && chars < cursor; ++bytes) if ((static_cast<unsigned char>(raw[bytes]) & 0xc0) != 0x80) ++chars;
    while (bytes < raw.size() && (static_cast<unsigned char>(raw[bytes]) & 0xc0) == 0x80) ++bytes;
    preedit.setCursor(static_cast<int>(bytes));
    const auto &items = frame.at("candidates").at("items");
    if (!items.is_array() || items.size() > 9) throw std::runtime_error("candidate count");
    auto watched = context->watch();
    const auto lifecycle = session->lifecycle;
    auto current = [&] {
        return watched.get() && session->opened && session->lifecycle == lifecycle
            && session->revision == revision && session->displayIdentity == identity;
    };
    auto list = std::make_unique<qingjian::List>(frame.at("page").get<int>(), frame.at("page_count").get<int>(), [this, alive = alive_, watched, revision, identity](bool next) {
        auto *ic = watched.get();
        if (*alive && ic && ic->hasFocus() && ic->propertyFor(&sessions_)->revision == revision)
            exchange(ic, {{"Page", {{"identity", identity}, {"next", next}}}});
    });
    list->setPageSize(9);
    list->setLabels({"1", "2", "3", "4", "5", "6", "7", "8", "9"});
    list->setLayoutHint(frame.value("layout", "horizontal") == "vertical" ? CandidateLayoutHint::Vertical : CandidateLayoutHint::Horizontal);
    nlohmann::json senses = nlohmann::json::array();
    size_t index = 0;
    for (const auto &item : items) {
        std::string annotation;
        auto text = item.at("text").get<std::string>();
        const auto &translation = item.at("translation");
        if (!text.empty() && translation.is_object() && !translation.at("senses").empty()) {
            const auto &sense = translation.at("senses").front();
            annotation = sense.at("text").get<std::string>();
            if (!annotation.empty()) senses.push_back({index, 0});
            if (sense.value("fresh", false)) annotation += " · 生";
        }
        list->append(std::make_unique<qingjian::Word>(text, annotation, [this, alive = alive_, watched, index, revision, identity](InputContext *ic) {
            if (*alive && ic && ic == watched.get() && ic->hasFocus() && ic->propertyFor(&sessions_)->revision == revision)
                exchange(ic, {{"Candidate", {{"identity", identity}, {"index", index}}}});
        }));
        ++index;
    }
    auto highlight = frame.at("highlight").get<size_t>();
    if (highlight < items.size()) list->setGlobalCursorIndex(static_cast<int>(highlight));
    auto &panel = context->inputPanel();
    panel.reset();
    const bool inlinePreedit = context->capabilityFlags().test(CapabilityFlag::Preedit) && session->preeditMode != "window";
    if (inlinePreedit) panel.setClientPreedit(preedit);
    if (session->preeditMode != "inline" || !inlinePreedit) panel.setPreedit(preedit);
    session->clientPreedit = inlinePreedit && !preedit.empty();
    if (!items.empty()) panel.setCandidateList(std::move(list));
    if (frame.at("notice").is_string()) panel.setAuxDown(Text(frame.at("notice").get<std::string>()));
    context->updatePreedit();
    if (!current()) return;
    context->updateUserInterface(UserInterfaceComponent::InputPanel);
    if (!current()) return;
    instance_->userInterfaceManager().flush();
    if (!current()) return;
    if (!session->opened || session->revision != revision || session->displayIdentity != identity || !context->hasFocus()
        || context->inputPanel().candidateList() == nullptr || instance_->userInterfaceManager().isVirtualKeyboardVisible())
        senses = nlohmann::json::array();
    if (!shared_->connection.send({{"DisplayAcknowledged", {{"session", session->id}, {"identity", identity}, {"senses", senses}}}}))
        throw std::runtime_error("display acknowledgment");
    watch(context, !preedit.empty());
}
void QingjianEngine::watch(InputContext *context, bool composing) {
    if (!composing) {
        if (polled_.get() == context) stopPolling();
        return;
    }
    polled_ = context->watch();
    if (!poller_) {
        poller_ = instance_->eventLoop().addTimeEvent(CLOCK_MONOTONIC, now(CLOCK_MONOTONIC) + POLL_INTERVAL_US, POLL_ACCURACY_US,
            [this](EventSourceTime *source, uint64_t) {
                poll();
                // 还在组句就续下一拍；停了就留着禁用，下次组句再启。
                if (polled_.isValid()) { source->setNextInterval(POLL_INTERVAL_US); source->setOneShot(); }
                else source->setEnabled(false);
                return true;
            });
        return;
    }
    poller_->setNextInterval(POLL_INTERVAL_US);
    poller_->setOneShot();
}
void QingjianEngine::stopPolling() {
    polled_.unwatch();
    if (poller_) poller_->setEnabled(false);
}
void QingjianEngine::poll() {
    auto *context = polled_.get();
    if (!context || shared_->clearing || !context->hasFocus() || !context->propertyFor(&sessions_)->opened) { stopPolling(); return; }
    auto *session = context->propertyFor(&sessions_);
    try {
        nlohmann::json response;
        if (!shared_->connection.send({{"Poll", {{"session", session->id}}}}, &response)) throw std::runtime_error("poll exchange");
        const auto &update = response.at("Update");
        const auto &identity = update.at("identity");
        if (update.at("session") != session->id || identity.at("generation") != session->generation
            || identity.at("context") != contextIdentity(context) || !identity.at("revision").is_number_unsigned())
            throw std::runtime_error("poll identity");
        const auto revision = identity.at("revision").get<uint64_t>();
        const uint64_t shown = session->displayIdentity.is_null() ? 0 : session->displayIdentity.at("revision").get<uint64_t>();
        if (revision < shown) throw std::runtime_error("poll identity");
        // 版本号没动：帧内容没变，不重画也不重复回报。
        if (revision == shown) return;
        session->displayIdentity = identity;
        render(context, update.at("frame"));
    } catch (const std::exception &) { disconnectAll(); }
}
}
