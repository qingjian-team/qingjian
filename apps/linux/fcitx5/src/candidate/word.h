//! 候选正文与第一条译词；选词仍回 Server 调 Engine::commit。
#pragma once
#include <fcitx/candidatelist.h>
#include <functional>
namespace qingjian {
class Word final : public fcitx::CandidateWord {
public:
    Word(std::string text, std::string comment, std::function<void(fcitx::InputContext *)> select)
        : CandidateWord(fcitx::Text(text)), select_(std::move(select)) {
        setPlaceHolder(text.empty());
        setComment(fcitx::Text(std::move(comment)));
    }
    void select(fcitx::InputContext *context) const override { if (!isPlaceHolder()) select_(context); }
private:
    /// 含帧版本检查的选择操作。
    std::function<void(fcitx::InputContext *)> select_;
};
}
