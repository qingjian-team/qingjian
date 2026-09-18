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
std::string contextIdentity(const InputContext *context) {
    std::ostringstream out;
    out << std::hex << std::setfill('0');
    for (auto byte : context->uuid()) out << std::setw(2) << static_cast<unsigned>(byte);
    return out.str();
}
}
QingjianEngine::QingjianEngine(AddonManager *manager)
    : instance_(manager->instance()), sessions_([](InputContext &) { return new qingjian::Session; }) {
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
        instance_->inputContextManager().foreach([this](InputContext *context) {
            auto *session = context->propertyFor(&sessions_);
            if (session->opened && !session->displayIdentity.is_null() && !session->connection.send({{"DisplayAcknowledged", {
                {"session", session->id}, {"identity", session->displayIdentity}, {"senses", nlohmann::json::array()}}}})) disconnect(context);
            return true;
        });
    });
}
void QingjianEngine::clear(InputContext *context) {
    const auto watched = context->watch();
    ++context->propertyFor(&sessions_)->revision;
    context->inputPanel().reset();
    context->updatePreedit();
    if (watched.get()) context->updateUserInterface(UserInterfaceComponent::InputPanel);
}
void QingjianEngine::disconnect(InputContext *context) {
    auto *session = context->propertyFor(&sessions_);
    session->socketWatcher.reset();
    session->connection.close();
    session->opened = false;
    session->displayIdentity = nullptr;
    session->capabilities = nullptr;
    session->clientPreedit = false;
    clear(context);
}
bool QingjianEngine::connect(InputContext *context) {
    auto *session = context->propertyFor(&sessions_);
    const auto watched = context->watch();
    if (session->opened) {
        if (session->focused != context->hasFocus()) {
            session->focused = context->hasFocus();
            exchange(context, {{"Focus", {{"focused", session->focused}}}}, false);
        }
        return watched.get() && session->opened;
    }
    try {
        if (!session->connection.open()) throw std::runtime_error("connect");
        ++session->generation;
        nlohmann::json response;
        if (!session->connection.send({{"OpenSession", {{"session", session->id}, {"app", context->program()}, {"protocol", 6}}}}, &response)
            || response.at("Update").at("session") != session->id
            || response.at("Update").at("linux_ui").at("version") != 2)
            throw std::runtime_error("protocol mismatch");
        if (!session->connection.send({{"LinuxHello", {{"version", 2}, {"generation", session->generation}, {"context", contextIdentity(context)}}}}, &response)
            || response.at("LinuxHello").at("version") != 2) throw std::runtime_error("linux handshake");
        session->preeditMode = response.at("LinuxHello").at("preedit").get<std::string>();
        if (session->preeditMode != "both" && session->preeditMode != "window" && session->preeditMode != "inline")
            throw std::runtime_error("preedit mode");
        session->opened = true;
        session->socketWatcher = instance_->eventLoop().addIOEvent(session->connection.fd(), IOEventFlag::In,
            [this, context](EventSourceIO *, int fd, IOEventFlags) {
                char byte;
                auto count = recv(fd, &byte, 1, MSG_PEEK | MSG_DONTWAIT);
                if (count >= 0 || (errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR)) disconnect(context);
                return true;
            });
        if (!syncPrivacy(context)) return false;
        session->focused = context->hasFocus();
        exchange(context, {{"Focus", {{"focused", session->focused}}}}, false);
        return watched.get() && session->opened;
    } catch (const std::exception &) { if (watched.get()) disconnect(context); return false; }
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
        return watched.get() && session->opened;
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
    try {
        nlohmann::json response;
        if (!session->opened || !session->connection.send({{"LinuxEvent", {{"session", session->id}, {"event", event}}}}, &response))
            throw std::runtime_error("event exchange");
        const auto &result = response.at("KeyResult");
        const auto &identity = result.at("identity");
        const auto outcome = result.at("outcome").get<std::string>();
        const auto &commit = result.at("commit");
        if (result.at("session") != session->id || (outcome != "Consumed" && outcome != "Passthrough")
            || (!commit.is_null() && !commit.is_string()) || identity.at("generation") != session->generation
            || identity.at("context") != contextIdentity(context) || !identity.at("revision").is_number_unsigned()
            || (!session->displayIdentity.is_null() && identity.at("revision").get<uint64_t>() <= session->displayIdentity.at("revision").get<uint64_t>()))
            throw std::runtime_error("response identity");
        session->displayIdentity = identity;
        // 显示 API 可同步重入能力 / 焦点 / Reset 事件。提交前再核对生命周期，
        // 撤销旧上下文结果时仍保留 Consumed，防止选词键再次透传。
        bool validDisplayFailure = false;
        if (display) {
            try { render(context, result.at("frame")); }
            catch (const std::exception &) {
                if (watched.get()) {
                    validDisplayFailure = session->displayIdentity == identity;
                    disconnect(context);
                }
            }
        }
        if (watched.get() && session->generation == generation && session->lifecycle == lifecycle
            && context->hasFocus() == focused && context->capabilityFlags() == capabilities
            && (session->displayIdentity == identity || validDisplayFailure) && commit.is_string())
            context->commitString(commit.get<std::string>());
        return outcome == "Consumed";
    } catch (const std::exception &) { if (watched.get()) disconnect(context); return false; }
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
    if (!connect(context) || !syncPrivacy(context)) return false;
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
    auto list = std::make_unique<qingjian::List>(frame.at("page").get<int>(), frame.at("page_count").get<int>(), [this, watched, revision, identity](bool next) {
        auto *ic = watched.get();
        if (ic && ic->hasFocus() && ic->propertyFor(&sessions_)->revision == revision)
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
        list->append(std::make_unique<qingjian::Word>(text, annotation, [this, watched, index, revision, identity](InputContext *ic) {
            if (ic && ic == watched.get() && ic->hasFocus() && ic->propertyFor(&sessions_)->revision == revision)
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
    if (!session->connection.send({{"DisplayAcknowledged", {{"session", session->id}, {"identity", identity}, {"senses", senses}}}}))
        throw std::runtime_error("display acknowledgment");
}
}
