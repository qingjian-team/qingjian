//! 候选回调身份；随 Fcitx4 候选列表一起释放。
#pragma once
#include <cstddef>
#include <cstdint>

namespace qingjian {
struct CandidateData {
    uint64_t session;

    uint64_t revision;

    size_t index;
};
}
