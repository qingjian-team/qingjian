//! 选区提交前核对文字与位置；先上屏后的同步重入不能覆盖新的翻译评审。
#include "qingjian.h"
#include "support/context.h"
#include "support/mock.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
#include <fcitx/surroundingtext.h>

int main(int argc, char **argv) {
    assert(argc == 2);
    const std::string scenario = argv[1];
    qingjian::test::Mock mock("selection");
    char program[] = "qingjian-test"; char disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    auto context = std::make_unique<Context>(instance.inputContextManager());
    context->setCapabilityFlags(fcitx::CapabilityFlags(fcitx::CapabilityFlag::Preedit) | fcitx::CapabilityFlag::SurroundingText);
    context->focusIn();
    context->surroundingText().setText("你好你好", 2, 0);
    auto translate = [&] {
        assert(engine.process(context.get(), fcitx::Key(FcitxKey_T, fcitx::KeyStates(fcitx::KeyState::Alt) | fcitx::KeyState::Shift)));
        assert(context->inputPanel().candidateList()->candidate(0).text().toString() == "hello");
    };
    translate();
    if (scenario == "changed-text") context->surroundingText().setText("再见你好", 2, 0);
    if (scenario == "changed-position") context->surroundingText().setText("你好你好", 4, 2);
    if (scenario == "commit-reentry") context->onCommit = [&] {
        assert(context->inputPanel().candidateList());
        translate();
    };
    assert(engine.process(context.get(), fcitx::Key(FcitxKey_Return)));
    const auto expected = scenario == "unchanged" || scenario == "commit-reentry" ? "hello" : "";
    assert(context->committed == expected);
    if (scenario == "commit-reentry") {
        assert(context->inputPanel().candidateList());
        context->surroundingText().setText("再见你好", 2, 0);
        assert(engine.process(context.get(), fcitx::Key(FcitxKey_Return)));
        assert(context->committed == "hello");
    }
    context.reset();
    auto timer = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 30000, 0,
        [&](fcitx::EventSourceTime *, uint64_t) { instance.eventLoop().exit(); return false; });
    instance.eventLoop().exec();
    mock.join();
}
