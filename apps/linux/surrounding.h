//! 从 Fcitx 提供的 Unicode 字符位置截取光标两侧文字，跳过选区。
#pragma once
#include <nlohmann/json.hpp>
#include <algorithm>
#include <cstddef>
#include <deque>
#include <string>
#include <string_view>

namespace qingjian {
inline std::string selected(std::string_view text, size_t cursor, size_t anchor) {
    if (cursor == anchor || (cursor > anchor ? cursor - anchor : anchor - cursor) > 500) return {};
    const size_t start = std::min(cursor, anchor), end = std::max(cursor, anchor);
    size_t chars = 0, begin = 0, finish = 0;
    for (size_t i = 0; i < text.size();) {
        if (chars == start) begin = i;
        if (chars == end) finish = i;
        const unsigned char lead = static_cast<unsigned char>(text[i]);
        const size_t width = lead < 0x80 ? 1 : lead >= 0xc2 && lead <= 0xdf ? 2
            : lead >= 0xe0 && lead <= 0xef ? 3 : lead >= 0xf0 && lead <= 0xf4 ? 4 : 0;
        if (!width || i + width > text.size()) return {};
        for (size_t j = 1; j < width; ++j)
            if ((static_cast<unsigned char>(text[i + j]) & 0xc0) != 0x80) return {};
        if (width >= 3) {
            const unsigned char second = static_cast<unsigned char>(text[i + 1]);
            if ((lead == 0xe0 && second < 0xa0) || (lead == 0xed && second >= 0xa0)
                || (lead == 0xf0 && second < 0x90) || (lead == 0xf4 && second >= 0x90)) return {};
        }
        i += width;
        ++chars;
    }
    if (chars == end) finish = text.size();
    if (end > chars) return {};
    return std::string(text.substr(begin, finish - begin));
}

inline nlohmann::json surrounding(std::string_view text, size_t cursor, size_t anchor) {
    constexpr size_t MAX_CHARS = 80;
    const size_t start = std::min(cursor, anchor);
    const size_t end = std::max(cursor, anchor);
    std::deque<size_t> recent;
    size_t chars = 0, beforeStart = 0, beforeEnd = 0, afterStart = 0, afterEnd = text.size();
    auto boundary = [&](size_t byte) {
        if (chars == start) {
            beforeStart = recent.empty() ? 0 : recent.front();
            beforeEnd = byte;
        }
        if (chars == end) afterStart = byte;
        if (chars >= end && chars - end == MAX_CHARS) afterEnd = byte;
    };
    for (size_t i = 0; i < text.size();) {
        boundary(i);
        if (chars < start) {
            recent.push_back(i);
            if (recent.size() > MAX_CHARS) recent.pop_front();
        }
        const unsigned char lead = static_cast<unsigned char>(text[i]);
        const size_t width = lead < 0x80 ? 1 : lead >= 0xc2 && lead <= 0xdf ? 2
            : lead >= 0xe0 && lead <= 0xef ? 3 : lead >= 0xf0 && lead <= 0xf4 ? 4 : 0;
        if (!width || i + width > text.size()) return nullptr;
        for (size_t j = 1; j < width; ++j)
            if ((static_cast<unsigned char>(text[i + j]) & 0xc0) != 0x80) return nullptr;
        if (width >= 3) {
            const unsigned char second = static_cast<unsigned char>(text[i + 1]);
            if ((lead == 0xe0 && second < 0xa0) || (lead == 0xed && second >= 0xa0)
                || (lead == 0xf0 && second < 0x90) || (lead == 0xf4 && second >= 0x90)) return nullptr;
        }
        i += width;
        ++chars;
    }
    boundary(text.size());
    if (cursor > chars || anchor > chars) return nullptr;
    return {{"before", std::string(text.substr(beforeStart, beforeEnd - beforeStart))},
            {"after", std::string(text.substr(afterStart, afterEnd - afterStart))}};
}
}
