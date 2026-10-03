//! Fcitx4 默认面板与 Linux Server 的协议适配。
#include "engine.h"
#include "candidate/data.h"
#include "key.h"
#include "primary_selection.h"
#include "../../surrounding.h"
#include <fcitx/candidate.h>
#include <fcitx/fcitx.h>
#include <fcitx/ime.h>
#include <fcitx/ui.h>
#include <fcitx-utils/utf8.h>
#include <cstring>
#include <cstdlib>
#include <stdexcept>

namespace qingjian {
namespace {
Engine *engine(void *arg) { return static_cast<Engine *>(arg); }
bool active(FcitxInstance *instance) {
    auto *im = FcitxInstanceGetCurrentIM(instance);
    return im && im->uniqueName && std::strcmp(im->uniqueName, "qingjian") == 0;
}
void onFocus(void *arg) { engine(arg)->focus(true); }
void onBlur(void *arg) { engine(arg)->focus(false); }
void onPoll(void *arg) { engine(arg)->poll(); }
void onRetired(void *arg) { engine(arg)->flushRetired(); }
INPUT_RETURN_VALUE onChoose(void *arg, FcitxCandidateWord *word) { return engine(arg)->choose(word); }
boolean onPage(void *arg, boolean previous) { return engine(arg)->page(previous); }
void onReset(void *arg) { engine(arg)->reset(); }
void onClose(void *arg, FcitxIMCloseEventType reason) { engine(arg)->close(reason); }
INPUT_RETURN_VALUE onPress(void *arg, FcitxKeySym sym, unsigned int state) { return engine(arg)->key(sym, state, false); }
INPUT_RETURN_VALUE onRelease(void *arg, FcitxKeySym sym, unsigned int state) { return engine(arg)->key(sym, state, true); }
boolean onInit(void *arg) { engine(arg)->initialize(); return true; }
void *create(FcitxInstance *instance) {
    auto *result = new Engine(instance);
    FcitxIMIFace iface{};
    iface.Init = onInit;
    iface.ResetIM = onReset;
    iface.DoInput = onPress;
    iface.DoReleaseInput = onRelease;
    iface.OnClose = onClose;
    FcitxInstanceRegisterIMv2(instance, result, "qingjian", "青简", "qingjian", iface, 10, "zh_CN");
    FcitxInstanceRegisterInputFocusHook(instance, {onFocus, result});
    FcitxInstanceRegisterInputUnFocusHook(instance, {onBlur, result});
    return result;
}
void destroy(void *arg) { delete engine(arg); }
}

Engine::Engine(FcitxInstance *instance) : instance_(instance) {
    dataSlot_ = FcitxInstanceAllocDataForIC(instance, allocate, copy, free, this);
}
Engine::~Engine() {
    stopPolling();
    if (retireTimer_) FcitxInstanceRemoveTimeoutById(instance_, retireTimer_);
    connection_.close();
}
void *Engine::allocate(void *arg) {
    auto *self = engine(arg);
    auto *session = new Session;
    session->id = self->nextSession_++;
    self->sessions_.insert(session);
    return session;
}
void *Engine::copy(void *arg, void *, void *) { return allocate(arg); }
void Engine::free(void *arg, void *data) {
    auto *self = engine(arg);
    auto *session = static_cast<Session *>(data);
    if (!session) return;
    self->retire(session);
    self->sessions_.erase(session);
    delete session;
}
void Engine::retire(Session *session) {
    if (polled_ == session) stopPolling();
    if (session->opened) {
        retired_.push_back(session->id);
        if (connection_.connected() && !retireTimer_)
            retireTimer_ = FcitxInstanceAddTimeout(instance_, 1, onRetired, this);
    }
}
Session *Engine::current() {
    auto *ic = FcitxInstanceGetCurrentIC(instance_);
    if (!ic || dataSlot_ < 0) return nullptr;
    auto *session = static_cast<Session *>(FcitxInstanceGetICData(instance_, ic, dataSlot_));
    if (session) session->context = ic;
    return session;
}
bool Engine::live(Session *session) const { return sessions_.contains(session); }
std::string Engine::identity(const Session *session) const { return "fcitx4-" + std::to_string(session->id); }
void Engine::flushRetired() {
    if (retireTimer_) FcitxInstanceRemoveTimeoutById(instance_, retireTimer_);
    retireTimer_ = 0;
    if (!connection_.connected()) { retired_.clear(); return; }
    for (auto id : retired_) {
        if (!connection_.send({{"CloseSession", {{"session", id}}}})) {
            disconnect();
            return;
        }
    }
    retired_.clear();
}
void Engine::disconnect() {
    stopPolling();
    if (retireTimer_) FcitxInstanceRemoveTimeoutById(instance_, retireTimer_);
    retireTimer_ = 0;
    connection_.close();
    retired_.clear();
    for (auto *session : sessions_) {
        session->opened = false;
        session->focused = false;
        session->identity = nullptr;
        session->capabilities = nullptr;
        session->composing = false;
    }
    if (auto *session = current()) clear(session);
}
void Engine::clear(Session *session) {
    if (!live(session) || !session->context || FcitxInstanceGetCurrentIC(instance_) != session->context) return;
    session->clientPreedit = false;
    session->selectionText.clear();
    session->composing = false;
    auto *input = FcitxInstanceGetInputState(instance_);
    FcitxMessagesSetMessageCount(FcitxInputStateGetPreedit(input), 0);
    FcitxMessagesSetMessageCount(FcitxInputStateGetClientPreedit(input), 0);
    FcitxMessagesSetMessageCount(FcitxInputStateGetAuxDown(input), 0);
    FcitxCandidateWordReset(FcitxInputStateGetCandidateList(input));
    FcitxInstanceUpdatePreedit(instance_, session->context);
    FcitxUIUpdateInputWindow(instance_);
}
std::string Engine::selection(Session *session, unsigned int *cursor, unsigned int *anchor) {
    if (!session->context || (session->context->contextCaps & CAPACITY_PASSWORD)) return {};
    char *text = nullptr;
    unsigned int caret = 0, mark = 0;
    const bool available = (session->context->contextCaps & CAPACITY_SURROUNDING_TEXT)
        && FcitxInstanceGetSurroundingText(instance_, session->context, &text, &caret, &mark);
    std::string result = available && text ? selected(text, caret, mark) : std::string();
    std::free(text);
    if (!available) result = primarySelection();
    if (cursor) *cursor = caret;
    if (anchor) *anchor = mark;
    return result;
}
bool Engine::connect(Session *session) {
    if (session->opened) return true;
    try {
        if (!connection_.connected()) {
            if (!connection_.open()) return false;
            ++generation_;
            retired_.clear();
        }
        flushRetired();
        nlohmann::json response;
        session->generation = generation_;
        if (!connection_.send({{"OpenSession", {{"session", session->id}, {"app", ""}, {"protocol", 7}}}}, &response)
            || response.at("Update").at("session") != session->id
            || response.at("Update").at("linux_ui").at("version") != 3)
            throw std::runtime_error("open session");
        if (!connection_.send({{"LinuxHello", {{"version", 3}, {"session", session->id}, {"generation", session->generation}, {"context", identity(session)}}}}, &response)
            || response.at("LinuxHello").at("version") != 3
            || response.at("LinuxHello").at("session") != session->id)
            throw std::runtime_error("linux hello");
        session->preeditMode = response.at("LinuxHello").at("preedit").get<std::string>();
        if (session->preeditMode != "both" && session->preeditMode != "inline" && session->preeditMode != "window")
            throw std::runtime_error("preedit mode");
        session->opened = true;
        if (!syncCapabilities(session)) return false;
        session->focused = true;
        exchange(session, {{"Focus", {{"focused", true}}}}, false);
        return live(session) && session->opened;
    } catch (const std::exception &) { disconnect(); return false; }
}
bool Engine::syncCapabilities(Session *session) {
    if (!live(session) || !session->context) return false;
    // Fcitx4 只向插件提供密码标记；没有与 Fcitx5 Sensitive/Disable 等价的位。
    nlohmann::json caps = {{"sensitive", false},
        {"password", (session->context->contextCaps & CAPACITY_PASSWORD) != 0}, {"disabled", false}};
    if (session->capabilities != caps) {
        exchange(session, {{"Capabilities", caps}}, false);
        if (!live(session) || !session->opened) return false;
        session->capabilities = caps;
        ++session->lifecycle;
        clear(session);
    }
    if (caps.at("password").get<bool>()) {
        if (session->opened) {
            retire(session);
            session->opened = false;
            session->id = nextSession_++;
        }
        clear(session);
        return false;
    }
    return session->opened;
}
bool Engine::exchange(Session *session, const nlohmann::json &event, bool display) {
    if (!live(session) || !session->opened) return false;
    const auto generation = session->generation;
    const auto lifecycle = session->lifecycle;
    bool consumed = false;
    try {
        nlohmann::json response;
        if (!connection_.send({{"LinuxEvent", {{"session", session->id}, {"event", event}}}}, &response))
            throw std::runtime_error("event exchange");
        if (response.contains("Ignored")) return true;
        if (response.contains("RequestSelection")) {
            const auto &request = response.at("RequestSelection");
            if (request.at("session") != session->id || !request.at("request").is_number_unsigned())
                throw std::runtime_error("selection request");
            auto text = selection(session, &session->selectionCursor, &session->selectionAnchor);
            session->selectionText = text;
            if (!connection_.send({{"LinuxEvent", {{"session", session->id}, {"event", {{"Selection", {
                {"request", request.at("request")}, {"text", text}}}}}}}}, &response))
                throw std::runtime_error("selection exchange");
        }
        const auto &result = response.at("KeyResult");
        const auto &id = result.at("identity");
        const auto &outcome = result.at("outcome").get_ref<const std::string &>();
        if (result.at("session") != session->id || id.at("generation") != session->generation
            || id.at("context") != identity(session) || !id.at("revision").is_number_unsigned()
            || (outcome != "Consumed" && outcome != "Passthrough"))
            throw std::runtime_error("response identity");
        consumed = outcome == "Consumed";
        session->identity = id;
        if (display) render(session, result.at("frame"));
        if (live(session) && session->opened && session->generation == generation
            && session->lifecycle == lifecycle && result.at("commit").is_string()
            && session->context && FcitxInstanceGetCurrentIC(instance_) == session->context) {
            unsigned int cursor = 0, anchor = 0;
            if (session->selectionText.empty() || (selection(session, &cursor, &anchor) == session->selectionText
                && cursor == session->selectionCursor && anchor == session->selectionAnchor))
                FcitxInstanceCommitString(instance_, session->context, result.at("commit").get_ref<const std::string &>().c_str());
        }
        if (result.at("frame").at("candidates").at("items").empty()) session->selectionText.clear();
        return consumed;
    } catch (const std::exception &) { disconnect(); return consumed; }
}
void Engine::render(Session *session, const nlohmann::json &frame) {
    if (!live(session) || !session->context || FcitxInstanceGetCurrentIC(instance_) != session->context) return;
    auto *input = FcitxInstanceGetInputState(instance_);
    auto *preedit = FcitxInputStateGetPreedit(input);
    auto *client = FcitxInputStateGetClientPreedit(input);
    auto *aux = FcitxInputStateGetAuxDown(input);
    auto *list = FcitxInputStateGetCandidateList(input);
    FcitxMessagesSetMessageCount(preedit, 0);
    FcitxMessagesSetMessageCount(client, 0);
    FcitxMessagesSetMessageCount(aux, 0);
    FcitxCandidateWordReset(list);
    std::string raw;
    for (const auto &segment : frame.at("preedit")) raw += segment.at("text").get<std::string>();
    const bool inlinePreedit = (session->context->contextCaps & CAPACITY_PREEDIT)
        && FcitxInstanceICSupportPreedit(instance_, session->context) && session->preeditMode != "window";
    if (!raw.empty()) {
        if (inlinePreedit) FcitxMessagesAddMessageAtLast(client, MSG_INPUT, "%s", raw.c_str());
        if (!inlinePreedit || session->preeditMode != "inline")
            FcitxMessagesAddMessageAtLast(preedit, MSG_INPUT, "%s", raw.c_str());
    }
    session->clientPreedit = inlinePreedit && !raw.empty();
    size_t byteCursor = 0;
    for (size_t chars = 0; byteCursor < raw.size() && chars < frame.at("cursor").get<size_t>(); ++byteCursor)
        if ((static_cast<unsigned char>(raw[byteCursor]) & 0xc0) != 0x80) ++chars;
    while (byteCursor < raw.size() && (static_cast<unsigned char>(raw[byteCursor]) & 0xc0) == 0x80) ++byteCursor;
    FcitxInputStateSetCursorPos(input, static_cast<int>(byteCursor));
    FcitxInputStateSetClientCursorPos(input, static_cast<int>(byteCursor));
    const auto &items = frame.at("candidates").at("items");
    if (!items.is_array() || items.size() > 9) throw std::runtime_error("candidate count");
    FcitxCandidateWordSetPageSize(list, 9);
    FcitxCandidateWordSetChoose(list, "123456789");
    FcitxCandidateWordSetLayoutHint(list, frame.value("layout", "horizontal") == "vertical" ? CLH_Vertical : CLH_Horizontal);
    FcitxCandidateWordSetOverridePaging(list, frame.at("page").get<int>() > 0,
        frame.at("page").get<int>() + 1 < frame.at("page_count").get<int>(), onPage, this, nullptr);
    nlohmann::json senses = nlohmann::json::array();
    for (size_t index = 0; index < items.size(); ++index) {
        const auto &item = items[index];
        FcitxCandidateWord word{};
        std::string annotation;
        const auto &translation = item.at("translation");
        if (translation.is_object() && !translation.at("senses").empty()) {
            const auto &sense = translation.at("senses").front();
            annotation = sense.at("text").get<std::string>();
            if (!annotation.empty()) senses.push_back({index, 0});
            if (sense.value("fresh", false)) annotation += " · 生";
        }
        if (item.at("kind") == "Cloud") annotation = "☁ " + annotation;
        word.strWord = ::strdup(item.at("text").get_ref<const std::string &>().c_str());
        word.strExtra = ::strdup(annotation.c_str());
        word.callback = onChoose;
        word.owner = this;
        auto *data = static_cast<CandidateData *>(std::malloc(sizeof(CandidateData)));
        if (!word.strWord || !word.strExtra || !data) {
            std::free(word.strWord);
            std::free(word.strExtra);
            std::free(data);
            throw std::bad_alloc();
        }
        *data = {session->id, session->identity.at("revision").get<uint64_t>(), index};
        word.priv = data;
        word.wordType = MSG_OTHER;
        word.extraType = MSG_OTHER;
        FcitxCandidateWordAppend(list, &word);
    }
    if (frame.at("highlight").get<size_t>() < items.size())
        FcitxCandidateWordSetFocus(list, frame.at("highlight").get<int>());
    if (frame.at("sentence").is_string())
        FcitxMessagesAddMessageAtLast(aux, MSG_TIPS, "☁ %s · Tab", frame.at("sentence").get_ref<const std::string &>().c_str());
    if (frame.at("notice").is_string())
        FcitxMessagesAddMessageAtLast(aux, MSG_TIPS, "%s", frame.at("notice").get_ref<const std::string &>().c_str());
    FcitxInstanceUpdatePreedit(instance_, session->context);
    FcitxUIUpdateInputWindow(instance_);
    session->composing = !raw.empty() || frame.value("translation_pending", false);
    if (session->composing && !timer_) {
        polled_ = session;
        timer_ = FcitxInstanceAddTimeout(instance_, 80, onPoll, this);
    } else if (!session->composing && polled_ == session) stopPolling();
    if (!connection_.send({{"DisplayAcknowledged", {{"session", session->id},
            {"identity", session->identity}, {"senses", senses}}}}))
        throw std::runtime_error("display acknowledgment");
}
void Engine::stopPolling() {
    if (timer_) FcitxInstanceRemoveTimeoutById(instance_, timer_);
    timer_ = 0;
    polled_ = nullptr;
}
void Engine::poll() {
    timer_ = 0;
    auto *session = polled_;
    if (!session || !live(session) || !session->opened || !session->focused
        || FcitxInstanceGetCurrentIC(instance_) != session->context) { stopPolling(); return; }
    try {
        nlohmann::json response;
        if (!connection_.send({{"Poll", {{"session", session->id}}}}, &response)) throw std::runtime_error("poll exchange");
        const auto &update = response.at("Update");
        const auto &id = update.at("identity");
        if (update.at("session") != session->id || id.at("generation") != session->generation
            || id.at("context") != identity(session)) throw std::runtime_error("poll identity");
        if (id != session->identity) {
            session->identity = id;
            render(session, update.at("frame"));
        }
        if (update.at("frame").at("candidates").at("items").empty()) session->selectionText.clear();
        if (!timer_ && session->composing && polled_ == session)
            timer_ = FcitxInstanceAddTimeout(instance_, 80, onPoll, this);
    } catch (const std::exception &) { disconnect(); }
}
void Engine::initialize() {
    if (auto *session = current()) { ++session->lifecycle; session->focused = true; }
}
void Engine::reset() {
    if (auto *session = current()) {
        ++session->lifecycle;
        if (session->opened) exchange(session, "Reset", false);
        clear(session);
    }
}
void Engine::close(FcitxIMCloseEventType reason) {
    if (auto *session = current()) {
        ++session->lifecycle;
        if (session->opened) exchange(session, {{"Deactivate", {
            {"focus_out", reason == CET_LostFocus}, {"client_preedit", session->clientPreedit},
            {"capability_changed", false}}}}, false);
        clear(session);
        session->focused = false;
    }
}
void Engine::focus(bool focused) {
    if (!active(instance_)) return;
    if (auto *session = current()) {
        ++session->lifecycle;
        if (session->opened) exchange(session, {{"Focus", {{"focused", focused}}}}, false);
        session->focused = focused;
        if (!focused) clear(session);
    }
}
INPUT_RETURN_VALUE Engine::key(FcitxKeySym sym, unsigned int state, bool release) {
    auto *session = current();
    if (!session) return IRV_DONOT_PROCESS;
    if (session->context->contextCaps & CAPACITY_PASSWORD) {
        if (session->opened) syncCapabilities(session);
        return IRV_DONOT_PROCESS;
    }
    if (!connect(session) || !syncCapabilities(session)) return IRV_DONOT_PROCESS;
    nlohmann::json nearby = nullptr;
    if (!release && (session->context->contextCaps & CAPACITY_SURROUNDING_TEXT)) {
        char *text = nullptr;
        unsigned int cursor = 0, anchor = 0;
        const bool available = FcitxInstanceGetSurroundingText(instance_, session->context, &text, &cursor, &anchor);
        if (available && text) nearby = surrounding(text, cursor, anchor);
        std::free(text);
    }
    bool consumed = exchange(session, {{"Key", {{"event", mapKey(sym, state)}, {"release", release},
        {"surrounding", nearby}, {"selection_supported", true}}}});
    return consumed ? IRV_DO_NOTHING : IRV_DONOT_PROCESS;
}
INPUT_RETURN_VALUE Engine::choose(FcitxCandidateWord *candidate) {
    auto *session = current();
    if (!session || !session->opened || !candidate || candidate->owner != this || !candidate->priv)
        return IRV_DONOT_PROCESS;
    auto *data = static_cast<CandidateData *>(candidate->priv);
    if (data->session != session->id || session->identity.at("revision") != data->revision)
        return IRV_DONOT_PROCESS;
    if (exchange(session, {{"Candidate", {{"identity", session->identity}, {"index", data->index}}}}))
        return IRV_DO_NOTHING;
    return IRV_DONOT_PROCESS;
}
boolean Engine::page(boolean previous) {
    auto *session = current();
    if (!session || !session->opened) return false;
    return exchange(session, {{"Page", {{"identity", session->identity}, {"next", !previous}}}});
}
}

extern "C" {
FCITX_EXPORT_API int ABI_VERSION = FCITX_ABI_VERSION;
FCITX_EXPORT_API FcitxIMClass ime = {qingjian::create, qingjian::destroy};
}
