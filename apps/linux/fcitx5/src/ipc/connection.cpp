//! 长度前缀显式使用小端；非阻塞 fd 内部同步 poll，共享 200ms 截止时间，耗时需与 UI 分开。
#include "connection.h"
#include <array>
#include <chrono>
#include <cstdlib>
#include <cstring>
#include <poll.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

namespace {
using Clock = std::chrono::steady_clock;
bool transfer(int fd, void *buffer, size_t length, bool write, Clock::time_point deadline) {
    auto *data = static_cast<char *>(buffer);
    while (length) {
        auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(deadline - Clock::now()).count();
        if (remaining <= 0) return false;
        pollfd pfd{fd, static_cast<short>(write ? POLLOUT : POLLIN), 0};
        int ready = poll(&pfd, 1, static_cast<int>(remaining));
        if (ready < 0 && errno == EINTR) continue;
        if (ready <= 0 || (pfd.revents & (POLLERR | POLLNVAL))) return false;
        auto size = write ? ::send(fd, data, length, MSG_NOSIGNAL) : ::recv(fd, data, length, 0);
        if (size < 0 && (errno == EINTR || errno == EAGAIN)) continue;
        if (size <= 0) return false;
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
    if (::connect(fd_, reinterpret_cast<sockaddr *>(&address), sizeof(address)) < 0) { close(); return false; }
    ucred credentials{};
    socklen_t size = sizeof(credentials);
    if (getsockopt(fd_, SOL_SOCKET, SO_PEERCRED, &credentials, &size) < 0 || credentials.uid != geteuid()) { close(); return false; }
    return true;
}
bool Connection::send(const nlohmann::json &message, nlohmann::json *response) {
    if (!connected()) return false;
    auto deadline = Clock::now() + std::chrono::milliseconds(200);
    std::string body = message.dump();
    uint32_t length = static_cast<uint32_t>(body.size());
    std::array<unsigned char, 4> prefix{static_cast<unsigned char>(length), static_cast<unsigned char>(length >> 8), static_cast<unsigned char>(length >> 16), static_cast<unsigned char>(length >> 24)};
    bool ok = transfer(fd_, prefix.data(), 4, true, deadline) && transfer(fd_, body.data(), body.size(), true, deadline);
    if (ok && response) {
        ok = transfer(fd_, prefix.data(), 4, false, deadline);
        length = uint32_t(prefix[0]) | (uint32_t(prefix[1]) << 8) | (uint32_t(prefix[2]) << 16) | (uint32_t(prefix[3]) << 24);
        ok = ok && length > 0 && length <= 16 * 1024 * 1024;
        if (ok) {
            body.resize(length);
            ok = transfer(fd_, body.data(), length, false, deadline);
            if (ok) { *response = nlohmann::json::parse(body, nullptr, false); ok = !response->is_discarded(); }
        }
    }
    if (!ok) close();
    return ok;
}
}
