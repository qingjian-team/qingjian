//! 框架真实事件次序：失焦只交付一次，能力与 Shift 事实完整转发。
#include "qingjian.h"
#include "support/context.h"
#include "support/mock.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
#include <fcitx-utils/log.h>
#include <sstream>
int main(int argc, char **argv) {
    assert(argc == 2);
    const std::string scenario = argv[1];
    qingjian::test::Mock mock("both");
    char program[] = "qingjian-test"; char disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    std::ostringstream diagnostics;
    auto &originalLog = fcitx::Log::logStream();
    fcitx::Log::setLogStream(diagnostics);
    fcitx::QingjianEngine engine(&instance.addonManager());
    fcitx::InputMethodEntry entry("qingjian", "qingjian", "zh_CN", "qingjian");
    auto context = std::make_unique<Context>(instance.inputContextManager());
    using Flag = fcitx::CapabilityFlag;
    fcitx::CapabilityFlags capabilities = Flag::Preedit;
    if (scenario == "focusout-panel") capabilities = Flag::SurroundingText;
    if (scenario == "focusout-client-owned") capabilities |= Flag::ClientUnfocusCommit;
    context->setCapabilityFlags(capabilities);
    context->focusIn();
    if (scenario != "shift-empty") assert(engine.process(context.get(), fcitx::Key(FcitxKey_n)));
    if (scenario.rfind("focusout-", 0) == 0) {
        if (scenario == "focusout-client-owned") context->commitString(context->inputPanel().clientPreedit().toStringForCommit());
        context->focusOut();
        fcitx::FocusOutEvent event(context.get());
        engine.deactivate(entry, event);
        assert(context->committed == "ni");
    } else if (scenario == "capability-deactivate") {
        fcitx::InputContextSwitchInputMethodEvent event(fcitx::InputMethodSwitchedReason::CapabilityChanged, "qingjian", context.get());
        engine.deactivate(entry, event);
        assert(context->committed.empty());
    } else if (scenario == "reset") {
        fcitx::InputContextEvent event(context.get(), fcitx::EventType::InputContextReset);
        engine.reset(entry, event);
        assert(context->committed.empty() && context->inputPanel().empty());
    } else {
        context->setCapabilityFlags(scenario == "password" ? fcitx::CapabilityFlags(Flag::Password) :
            scenario == "sensitive" ? fcitx::CapabilityFlags(Flag::Sensitive) :
            scenario == "disabled" ? fcitx::CapabilityFlags(Flag::Disable) : fcitx::CapabilityFlags());
        if (scenario != "empty" && scenario != "shift-empty") assert(context->inputPanel().empty());
        if (scenario == "shift-empty") {
            fcitx::KeyEvent press(context.get(), fcitx::Key(FcitxKey_Shift_L), false);
            fcitx::KeyEvent release(context.get(), fcitx::Key(FcitxKey_Shift_L), true);
            engine.keyEvent(entry, press);
            engine.keyEvent(entry, release);
        }
    }
    std::unique_ptr<fcitx::EventSourceTime> delayedDestroy;
    if (scenario == "shift-empty") {
        delayedDestroy = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC,
            fcitx::now(CLOCK_MONOTONIC) + 100000, 1000,
            [&](fcitx::EventSourceTime *, uint64_t) { context.reset(); return false; });
    } else context.reset();
    auto timer = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 200000, 1000,
        [&](fcitx::EventSourceTime *, uint64_t) { instance.eventLoop().exit(); return false; });
    instance.eventLoop().exec();
    mock.join();
    fcitx::Log::setLogStream(originalLog);
    const auto logs = diagnostics.str();
    if (scenario.rfind("focusout-", 0) == 0)
        assert(logs.find("Qingjian lifecycle: reason=focus-out") != std::string::npos);
    else if (scenario == "capability-deactivate")
        assert(logs.find("Qingjian lifecycle: reason=deactivate-capability") != std::string::npos);
    else if (scenario == "reset")
        assert(logs.find("Qingjian lifecycle: reason=reset") != std::string::npos);
    else if (scenario == "sensitive" || scenario == "password" || scenario == "disabled")
        assert(logs.find("Qingjian lifecycle: reason=capability-change") != std::string::npos);
    assert(logs.find("preedit=ni") == std::string::npos);
    assert(logs.find("virtual_key") == std::string::npos);
    bool sawShiftPress = false, sawShiftRelease = false, sawCapabilityReason = false;
    Json lastCaps;
    for (const auto &message : mock.messages) {
        if (!message.contains("LinuxEvent")) continue;
        const auto &event = message.at("LinuxEvent").at("event");
        if (event.contains("Capabilities")) lastCaps = event.at("Capabilities");
        if (event.contains("Key") && event.at("Key").at("event").at("virtual_key") == 16) {
            if (event.at("Key").at("release").get<bool>()) sawShiftRelease = true;
            else sawShiftPress = true;
        }
        if (event.contains("Deactivate")) sawCapabilityReason |= event.at("Deactivate").at("capability_changed").get<bool>();
    }
    assert(lastCaps.at("sensitive") == (scenario == "sensitive"));
    assert(lastCaps.at("password") == (scenario == "password"));
    assert(lastCaps.at("disabled") == (scenario == "disabled"));
    if (scenario == "shift-empty") assert(sawShiftPress && sawShiftRelease);
    if (scenario == "capability-deactivate") assert(sawCapabilityReason);
}
