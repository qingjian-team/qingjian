//! 可观测应用上屏与可重入 preedit 回调的真实 Fcitx 上下文。
#pragma once
#include <fcitx/inputcontext.h>
#include <fcitx/inputpanel.h>
#include <functional>
#include <utility>
class Input final : public fcitx::InputContext {
public:
    explicit Input(fcitx::InputContextManager &manager) : InputContext(manager, "real-test") { created(); setCapabilityFlags(fcitx::CapabilityFlag::Preedit); }
    ~Input() override { destroy(); }
    const char *frontend() const override { return "test"; }
    std::string committed;

    std::function<void()> preeditHook;

    std::function<void()> commitHook;

    std::string preedit() const { return inputPanel().clientPreedit().toString(); }
protected:
    void commitStringImpl(const std::string &text) override {
        committed += text;
        // 回调只触发一次。不用 std::move：libc++ 不保证移走后的 std::function 变空
        auto callback = std::exchange(commitHook, nullptr);
        if (callback) callback();
    }
    void deleteSurroundingTextImpl(int, unsigned int) override {}
    void forwardKeyImpl(const fcitx::ForwardKeyEvent &) override {}
    void updatePreeditImpl() override { auto callback = std::exchange(preeditHook, nullptr); if (callback) callback(); }
};
