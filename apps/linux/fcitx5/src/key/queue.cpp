//! 框架事件按顺序异步收发；只在 Server 确认直通后转发原按键，不重放未确认请求。
#include "qingjian.h"
#include "mapping.h"
#include "../../../surrounding.h"
#include <fcitx/inputpanel.h>
#include <fcitx-utils/log.h>
#include <iomanip>
#include <sstream>
#include <stdexcept>
#include <utility>

namespace fcitx {
namespace {
std::string identityOf(const InputContext *context) {
    std::ostringstream out;
    out << std::hex << std::setfill('0');
    for (auto byte : context->uuid()) out << std::setw(2) << static_cast<unsigned>(byte);
    return out.str();
}
nlohmann::json factsOf(const InputContext *context) {
    const auto caps = context->capabilityFlags();
    return {{"sensitive", caps.test(CapabilityFlag::Sensitive)},
        {"password", caps.test(CapabilityFlag::Password)}, {"disabled", caps.test(CapabilityFlag::Disable)}};
}
}
void QingjianEngine::queueKey(InputContext *context, const Key &key, bool release, int time) {
    nlohmann::json nearby = nullptr;
    if (!release && !context->capabilityFlags().test(CapabilityFlag::Sensitive)) {
        const auto &text = context->surroundingText();
        if (text.isValid()) nearby = qingjian::surrounding(text.text(), text.cursor(), text.anchor());
    }
    auto *session = context->propertyFor(&sessions_);
    enqueue({context->watch(), session->lifecycle, shared_->generation,
        {{"Key", {{"event", qingjian::mapKey(key)}, {"release", release},
            {"surrounding", nearby}, {"selection_supported", true}}}}, key, release, time,
        context->hasFocus()});
}
void QingjianEngine::queueEvent(InputContext *context, const nlohmann::json &event,
                                bool display, bool priority, bool wire) {
    auto *session = context->propertyFor(&sessions_);
    qingjian::PendingEvent pending{context->watch(), session->lifecycle, shared_->generation,
        event, std::nullopt, false, 0, context->hasFocus(), display, wire};
    enqueue(std::move(pending), priority);
}
bool QingjianEngine::notice(InputContext *context, const nlohmann::json &message) {
    if (!async_) return shared_->connection.send(message);
    queueEvent(context, message, false, true, true);
    return true;
}
void QingjianEngine::enqueue(qingjian::PendingEvent event, bool priority) {
    if (pending_.size() >= 1024) {
        disconnectAll("async-queue-limit");
        return;
    }
    if (priority) pending_.push_front(std::move(event));
    else pending_.push_back(std::move(event));
    pump();
}
void QingjianEngine::pump() {
    if (!async_ || busy_ || shared_->clearing) return;
    auto retired = std::exchange(shared_->retired, {});
    for (auto iterator = retired.rbegin(); iterator != retired.rend(); ++iterator) {
        qingjian::PendingEvent close;
        close.generation = shared_->generation;
        close.event = {{"CloseSession", {{"session", *iterator}}}};
        close.wire = true;
        pending_.push_front(std::move(close));
    }
    while (!pending_.empty()) {
        auto job = std::move(pending_.front());
        pending_.pop_front();
        auto *context = job.context.get();
        const bool close = job.wire && job.event.contains("CloseSession");
        const bool boundary = job.event == "Reset" || job.event.contains("Capabilities")
            || job.event.contains("Focus") || job.event.contains("Deactivate");
        if (job.generation != shared_->generation) continue;
        if (!close && (!context || (!boundary
            && (context->propertyFor(&sessions_)->lifecycle != job.lifecycle
                || context->hasFocus() != job.focused)))) continue;
        auto *session = context ? context->propertyFor(&sessions_) : nullptr;
        const auto id = session ? session->id : 0;
        const auto generation = shared_->generation;
        const auto identity = context ? identityOf(context) : std::string();
        const auto app = context ? context->program() : std::string();
        const auto facts = job.event.contains("Capabilities") ? job.event.at("Capabilities")
            : context ? factsOf(context) : nlohmann::json();
        const bool opening = !job.wire && !session->opened;
        const bool privacy = !job.wire && (opening || session->capabilities != facts);
        const bool focus = !job.wire && (opening || session->focused != job.focused);
        const bool reconnect = std::exchange(reconnect_, false);
        const auto body = job.event;
        const bool wire = job.wire, polling = job.poll, focused = job.focused;
        if (opening) { session->opening = true; session->generation = generation; }
        busy_ = true;
        async_->submit(
            [body, wire, polling, focused, id, generation, identity, app, facts, opening, privacy, focus, reconnect]
            (qingjian::Connection &connection) -> qingjian::AsyncConnection::Reply {
                if (reconnect) connection.close();
                if (wire) {
                    if (connection.connected() && !connection.send(body)) return std::nullopt;
                    return nlohmann::json::object();
                }
                if (!connection.open()) return std::nullopt;
                auto request = [&](const nlohmann::json &message) {
                    nlohmann::json response;
                    if (!connection.send(message, &response)) throw std::runtime_error("async IPC exchange");
                    return response;
                };
                auto event = [&](const nlohmann::json &body) {
                    auto response = request({{"LinuxEvent", {{"session", id}, {"event", body}}}});
                    return response;
                };
                auto verify = [&](const nlohmann::json &response) {
                    const auto &result = response.at("KeyResult");
                    const auto &stamp = result.at("identity");
                    if (result.at("session") != id || stamp.at("generation") != generation
                        || stamp.at("context") != identity || !stamp.at("revision").is_number_unsigned())
                        throw std::runtime_error("async setup identity");
                };
                std::string preedit;
                if (opening) {
                    const auto opened = request({{"OpenSession", {{"session", id}, {"app", app}, {"protocol", 9}}}});
                    if (opened.at("Update").at("session") != id
                        || opened.at("Update").at("linux_ui").at("version") != 3)
                        throw std::runtime_error("async protocol mismatch");
                    const auto hello = request({{"LinuxHello", {{"version", 3}, {"session", id},
                        {"generation", generation}, {"context", identity}}}}).at("LinuxHello");
                    if (hello.at("session") != id || hello.at("version") != 3)
                        throw std::runtime_error("async handshake identity");
                    preedit = hello.at("preedit").get<std::string>();
                    if (preedit != "both" && preedit != "window" && preedit != "inline")
                        throw std::runtime_error("async preedit mode");
                }
                if (privacy) verify(event({{"Capabilities", facts}}));
                if (focus) verify(event({{"Focus", {{"focused", focused}}}}));
                auto response = polling ? request({{"Poll", {{"session", id}}}}) : event(body);
                return nlohmann::json{{"response", std::move(response)}, {"preedit", preedit}};
            },
            [this, job, facts, generation, opening](qingjian::AsyncConnection::Reply reply) {
                auto *current = job.context.get();
                if (generation == shared_->generation) {
                    if (!reply) disconnectAll("async-exchange");
                    else if (!job.wire && current) {
                        auto *state = current->propertyFor(&sessions_);
                        state->opening = false;
                        state->opened = true;
                        state->generation = generation;
                        state->capabilities = facts;
                        state->focused = job.focused;
                        if (opening) state->preeditMode = reply->at("preedit").get<std::string>();
                        if (state->lifecycle == job.lifecycle && current->hasFocus() == job.focused) {
                            const auto &response = reply->at("response");
                            if (job.poll) {
                                if (polled_.get() == current) poll(&response);
                            } else {
                                const bool consumed = exchange(current, job.event, job.display, &response);
                                current = job.context.get();
                                if (!consumed && job.key && current && generation == shared_->generation
                                    && current->propertyFor(&sessions_)->lifecycle == job.lifecycle
                                    && current->hasFocus() == job.focused)
                                    current->forwardKey(*job.key, job.release, job.time);
                            }
                        }
                    }
                }
                busy_ = false;
                pump();
            });
        return;
    }
}
}
