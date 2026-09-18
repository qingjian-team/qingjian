//! 测试消息读写；断言只检查协议适配，业务在 Rust 测试覆盖。
#pragma once
#include <nlohmann/json.hpp>
#include <array>
#include <cassert>
#include <sys/socket.h>
using Json = nlohmann::json;
inline Json readMessage(int fd) {
    std::array<uint8_t, 4> prefix{};
    assert(recv(fd, prefix.data(), 4, MSG_WAITALL) == 4);
    uint32_t size = uint32_t(prefix[0]) | uint32_t(prefix[1]) << 8 | uint32_t(prefix[2]) << 16 | uint32_t(prefix[3]) << 24;
    assert(size > 0 && size < 100000);
    std::string body(size, '\0');
    assert(recv(fd, body.data(), size, MSG_WAITALL) == size);
    return Json::parse(body);
}
inline void writeMessage(int fd, const Json &message) {
    std::string body = message.dump();
    uint32_t size = body.size();
    std::array<uint8_t, 4> prefix{uint8_t(size), uint8_t(size >> 8), uint8_t(size >> 16), uint8_t(size >> 24)};
    assert(send(fd, prefix.data(), 4, MSG_NOSIGNAL) == 4);
    assert(send(fd, body.data(), size, MSG_NOSIGNAL) == size);
}
inline Json emptyFrame() {
    return Json::parse(R"({"preedit":[],"cursor":0,"candidates":{"items":[]},"highlight":0,"page":0,"page_count":1,"notice":null})");
}
