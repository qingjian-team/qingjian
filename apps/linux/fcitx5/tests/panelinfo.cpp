//! 框架写进面板的「输入法信息」（切输入法的一秒提示）不被插件的清屏清掉（#311）。
#include "qingjian.h"
#include "support/context.h"
#include "support/mock.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>

namespace {
/// 5.1.22 之前框架把提示写在 auxUp，之后写在 overlayMessage：两条插件都不能碰。
void assertInfo(const fcitx::InputPanel &panel) {
    assert(panel.auxUp().toString() == "青");
#ifdef QINGJIAN_HAS_OVERLAY_MESSAGE
    assert(panel.overlayMessage().toString() == "青");
#endif
}
}

int main() {
    qingjian::test::Mock mock("both");
    char program[] = "qingjian-test"; char disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    fcitx::InputMethodEntry entry("qingjian", "qingjian", "zh_CN", "qingjian");
    auto context = std::make_unique<Context>(instance.inputContextManager());
    context->setCapabilityFlags(fcitx::CapabilityFlag::Preedit);
    context->focusIn();
    context->inputPanel().setAuxUp(fcitx::Text("青"));
#ifdef QINGJIAN_HAS_OVERLAY_MESSAGE
    context->inputPanel().setOverlayMessage(fcitx::Text("青"));
#endif
    // 渲染一帧：候选进面板，插件只清自己的字段，提示留着。
    assert(engine.process(context.get(), fcitx::Key(FcitxKey_n)));
    assert(context->inputPanel().candidateList());
    assertInfo(context->inputPanel());
    // 失活清屏：拼音与候选收掉，提示仍留着，由框架自己的定时器收。
    fcitx::InputContextSwitchInputMethodEvent event(fcitx::InputMethodSwitchedReason::Other, "qingjian", context.get());
    engine.deactivate(entry, event);
    assert(!context->inputPanel().candidateList());
    assert(context->inputPanel().preedit().empty());
    assert(context->inputPanel().clientPreedit().empty());
    assertInfo(context->inputPanel());
    context.reset();
    // 会话随上下文销毁；跑一拍事件循环让连接收尾，Mock 才能退出。
    auto timer = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 30000, 0,
        [&](fcitx::EventSourceTime *, uint64_t) { instance.eventLoop().exit(); return false; });
    instance.eventLoop().exec();
    mock.join();
}
