//! 小端长度前缀与可取消的限时传输；生产收发在后台线程执行。
#include "connection.h"
#include <fcitx-utils/log.h>
#include <array>
#include <cerrno>
#include <chrono>
#include <cstdlib>
#include <cstring>
#include <poll.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#ifndef FCITX_WARN
#define FCITX_WARN() FCITX_WARNING()
#endif

namespace {
using Clock = std::chrono::steady_clock;
const char *operationName(const nlohmann::json &message) {
    const auto *event = &message;
    if (message.contains("LinuxEvent") && message.at("LinuxEvent").is_object()
        && message.at("LinuxEvent").contains("event"))
        event = &message.at("LinuxEvent").at("event");
    if (*event == "Reset") return "Reset";
    for (const auto *name : {"OpenSession", "CloseSession", "LinuxHello", "DisplayAcknowledged",
                            "Privacy", "Key", "Poll", "Commit", "Capabilities", "Focus",
                            "Candidate", "Selection", "Deactivate"})
        if (event->contains(name)) return name;
    return "unknown";
}
bool transfer(int fd, void *buffer, size_t length, bool write, Clock::time_point deadline,
              const char *&reason, int &systemError, const std::atomic<bool> *cancelled) {
    auto *data = static_cast<char *>(buffer);
    while (length) {
        if (cancelled && cancelled->load()) { reason = "cancelled"; return false; }
        auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(deadline - Clock::now()).count();
        if (remaining <= 0) { reason = "timeout"; return false; }
        pollfd pfd{fd, static_cast<short>(write ? POLLOUT : POLLIN), 0};
        int ready = poll(&pfd, 1, static_cast<int>(cancelled ? std::min<int64_t>(remaining, 20) : remaining));
        if (ready < 0 && errno == EINTR) continue;
        if (ready == 0) continue;
        if (ready < 0 || (pfd.revents & (POLLERR | POLLNVAL))) {
            reason = "poll-error";
            systemError = ready < 0 ? errno : 0;
            return false;
        }
        auto size = write ? ::send(fd, data, length, MSG_NOSIGNAL) : ::recv(fd, data, length, 0);
        if (size < 0 && (errno == EINTR || errno == EAGAIN)) continue;
        if (size <= 0) {
            reason = size == 0 ? "peer-closed" : "socket-error";
            systemError = size < 0 ? errno : 0;
            return false;
        }
        data += size;
        length -= static_cast<size_t>(size);
    }
    return true;
}
}
namespace qingjian {
Connection::~Connection() { close(); }
void Connection::close() { if (fd_ >= 0) ::close(fd_); fd_ = -1; }
bool Connection::open() {
    if (connected()) return true;
    fd_ = ::socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
    if (fd_ < 0) return false;
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    const char *custom = std::getenv("QINGJIAN_SOCKET");
    const char *runtime = std::getenv("XDG_RUNTIME_DIR");
    std::string path = custom ? custom : std::string(runtime && *runtime ? runtime : ("/tmp/qingjian-" + std::to_string(geteuid())).c_str()) + "/qingjian.sock";
    if (path.empty() || path[0] != '/' || path.size() >= sizeof(address.sun_path)) { close(); return false; }
    std::memcpy(address.sun_path, path.c_str(), path.size() + 1);
    if (::connect(fd_, reinterpret_cast<sockaddr *>(&address), sizeof(address)) < 0) {
        const auto systemError = errno;
        FCITX_WARN() << "Qingjian IPC connection failed: stage=connect errno=" << systemError;
        close(); return false;
    }
#if defined(__linux__)
    ucred credentials{};
    socklen_t size = sizeof(credentials);
    if (getsockopt(fd_, SOL_SOCKET, SO_PEERCRED, &credentials, &size) < 0 || credentials.uid != geteuid()) { close(); return false; }
#else
    // BSD 没有 SO_PEERCRED；glibc 没有 getpeereid，所以分开写
    uid_t uid = 0;
    gid_t gid = 0;
    if (getpeereid(fd_, &uid, &gid) < 0 || uid != geteuid()) { close(); return false; }
#endif
    return true;
}
bool Connection::send(const nlohmann::json &message, nlohmann::json *response) {
    if (!connected()) return false;
    const auto started = Clock::now();
    const auto deadline = started + timeout_;
    const char *stage = "write-prefix";
    const char *reason = "unknown";
    int systemError = 0;
    auto part = [&](void *buffer, size_t size, bool write, const char *name) {
        stage = name;
        return transfer(fd_, buffer, size, write, deadline, reason, systemError, cancelled_);
    };
    std::string body = message.dump();
    uint32_t length = static_cast<uint32_t>(body.size());
    std::array<unsigned char, 4> prefix{static_cast<unsigned char>(length), static_cast<unsigned char>(length >> 8), static_cast<unsigned char>(length >> 16), static_cast<unsigned char>(length >> 24)};
    bool ok = part(prefix.data(), 4, true, "write-prefix") && part(body.data(), body.size(), true, "write-body");
    if (ok && response) {
        ok = part(prefix.data(), 4, false, "read-prefix");
        length = uint32_t(prefix[0]) | (uint32_t(prefix[1]) << 8) | (uint32_t(prefix[2]) << 16) | (uint32_t(prefix[3]) << 24);
        if (ok && (length == 0 || length > 16 * 1024 * 1024)) {
            stage = "read-length";
            reason = "invalid-length";
        }
        ok = ok && length > 0 && length <= 16 * 1024 * 1024;
        if (ok) {
            body.resize(length);
            ok = part(body.data(), length, false, "read-body");
            if (ok) {
                *response = nlohmann::json::parse(body, nullptr, false);
                ok = !response->is_discarded();
                if (!ok) { stage = "parse-response"; reason = "invalid-json"; }
            }
        }
    }
    const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(Clock::now() - started).count();
    if (!ok) {
        FCITX_WARN() << "Qingjian IPC request failed: operation=" << operationName(message)
                     << " stage=" << stage << " reason=" << reason << " elapsed_ms=" << elapsed
                     << " errno=" << systemError;
        close();
    } else if (elapsed >= 100) {
        FCITX_WARN() << "Qingjian IPC request slow: operation=" << operationName(message) << " elapsed_ms=" << elapsed;
    }
    return ok;
}
}
