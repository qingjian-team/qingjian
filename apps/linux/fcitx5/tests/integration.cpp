//! 无会话总线加载、默认面板回报、候选点击和协议失效回归。
#include "qingjian.h"
#include "key/mapping.h"
#include "support/context.h"
#include "support/mock.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
int main(int argc, char **argv) {
    assert(argc == 2);
    const std::string mode = argv[1];
    qingjian::test::Mock mock(mode);
    auto escaped = qingjian::mapKey(fcitx::Key(FcitxKey_quotedbl));
    assert(Json::parse(escaped.dump()).at("character") == "\"");
    assert(qingjian::mapKey(fcitx::Key(FcitxKey_ISO_Left_Tab))["modifiers"]["shift"] == true);
    char program[] = "qingjian-test"; char disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    auto owned = std::make_unique<Context>(instance.inputContextManager());
    auto &context = *owned;
    context.setCapabilityFlags(fcitx::CapabilityFlag::Preedit);
    context.focusIn();
    const bool consumed = engine.process(&context, fcitx::Key(FcitxKey_n));
    if (mode == "mismatch") {
        assert(!consumed && context.inputPanel().empty() && context.committed.empty());
        return 0;
    }
    assert(consumed);
    assert(context.inputPanel().clientPreedit().empty() == (mode == "window"));
    assert(context.inputPanel().preedit().empty() == (mode == "inline"));
    auto candidates = context.inputPanel().candidateList();
    assert(candidates && candidates->size() == 3 && candidates->toPageable()->hasNext());
    assert(candidates->candidate(2).comment().toString() == "hello · 生");
    candidates->candidate(0).select(&context);
    assert(context.committed.empty());
    if (mode == "failure") {
        mock.join();
        auto timer = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 50000, 0,
            [&](fcitx::EventSourceTime *, uint64_t) { instance.eventLoop().exit(); return false; });
        instance.eventLoop().exec();
        assert(!context.inputPanel().candidateList() && context.inputPanel().clientPreedit().empty());
        candidates->candidate(2).select(&context);
        assert(context.committed.empty());
    } else {
        candidates->candidate(2).select(&context);
        assert(context.committed == "你好");
        candidates->candidate(2).select(&context);
        assert(context.committed == "你好");
    }
    owned.reset();
    if (mode != "failure") {
        auto timer = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 30000, 0,
            [&](fcitx::EventSourceTime *, uint64_t) { instance.eventLoop().exit(); return false; });
        instance.eventLoop().exec();
    }
    candidates->toPageable()->next();
    candidates->candidate(2).select(nullptr);
    mock.join();
    bool sawExposure = false;
    for (const auto &message : mock.messages) {
        if (message.contains("DisplayAcknowledged") && message.at("DisplayAcknowledged").at("senses") == Json::parse("[[2,0]]")) sawExposure = true;
    }
    assert(sawExposure);
}
