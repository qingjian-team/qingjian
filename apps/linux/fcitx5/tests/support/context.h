//! 默认面板回归使用的无应用输入上下文。
#pragma once
#include <fcitx/inputcontext.h>
#include <functional>
class Context final : public fcitx::InputContext {
public:
    explicit Context(fcitx::InputContextManager &manager) : InputContext(manager, "panel-test") { created(); }
    ~Context() override { destroy(); }
    const char *frontend() const override { return "test"; }
    std::string committed;

    std::function<void()> onPreedit;
protected:
    void commitStringImpl(const std::string &text) override { committed += text; }
    void deleteSurroundingTextImpl(int, unsigned int) override {}
    void forwardKeyImpl(const fcitx::ForwardKeyEvent &) override {}
    void updatePreeditImpl() override {
        auto callback = std::move(onPreedit);
        if (callback) callback();
    }
};
