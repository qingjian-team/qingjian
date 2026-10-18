//! Fcitx4 输入法壳：每个输入上下文有独立 Server 会话。
#pragma once
#include "session.h"
#include "../../fcitx5/src/ipc/connection.h"
#include <fcitx/candidate.h>
#include <fcitx/hook.h>
#include <fcitx/instance.h>
#include <nlohmann/json.hpp>
#include <string>
#include <unordered_set>
#include <vector>

namespace qingjian {
class Engine {
public:
    explicit Engine(FcitxInstance *instance);
    ~Engine();
    void initialize();
    void reset();
    void close(FcitxIMCloseEventType reason);
    void focus(bool focused);
    INPUT_RETURN_VALUE key(FcitxKeySym sym, unsigned int state, bool release);
    INPUT_RETURN_VALUE choose(FcitxCandidateWord *candidate);
    boolean page(boolean previous);
    void poll();
    void flushRetired();
    static void *allocate(void *arg);
    static void *copy(void *arg, void *, void *);
    static void free(void *arg, void *data);
    void retire(Session *session);
private:
    Session *current();
    bool live(Session *session) const;
    bool connect(Session *session);
    bool syncCapabilities(Session *session);
    bool exchange(Session *session, const nlohmann::json &event, bool display = true);
    std::string selection(Session *session, unsigned int *cursor, unsigned int *anchor);
    void render(Session *session, const nlohmann::json &frame);
    void clear(Session *session);
    void disconnect();
    void stopPolling();
    std::string identity(const Session *session) const;

    FcitxInstance *instance_;

    int dataSlot_ = -1;

    Connection connection_;

    uint64_t nextSession_ = 1;

    uint64_t generation_ = 0;

    uint64_t timer_ = 0;

    uint64_t retireTimer_ = 0;

    Session *polled_ = nullptr;

    std::unordered_set<Session *> sessions_;

    std::vector<uint64_t> retired_;
};
}
