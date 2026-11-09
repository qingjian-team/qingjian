//! 默认面板回归使用的无应用输入上下文。
#pragma once
#include <fcitx/inputcontext.h>
#include <functional>
#include <utility>
class Context final : public fcitx::InputContext {
public:
    explicit Context(fcitx::InputContextManager &manager) : InputContext(manager, "panel-test") { created(); }
    ~Context() override { destroy(); }
    const char *frontend() const override { return "test"; }
    std::string committed;

    std::function<void()> onPreedit;

    std::function<void()> onCommit;
protected:
    void commitStringImpl(const std::string &text) override {
        committed += text;
        auto callback = std::move(onCommit);
        if (callback) callback();
    }
    void deleteSurroundingTextImpl(int, unsigned int) override {}
    void forwardKeyImpl(const fcitx::ForwardKeyEvent &) override {}
    void updatePreeditImpl() override {
        // 回调只触发一次。不用 std::move：libc++ 不保证移走后的 std::function 变空
        auto callback = std::exchange(onPreedit, nullptr);
        if (callback) callback();
    }
};
