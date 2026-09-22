//! 组句期间轮询：Server 回新版本的换序帧就重画一次并回报，同版本不重画也不重复回报。
#include "qingjian.h"
#include "support/context.h"
#include "support/mock.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
int main() {
    qingjian::test::Mock mock("rescore");
    char program[] = "qingjian-test"; char disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    auto owned = std::make_unique<Context>(instance.inputContextManager());
    auto &context = *owned;
    context.setCapabilityFlags(fcitx::CapabilityFlag::Preedit);
    context.focusIn();
    assert(engine.process(&context, fcitx::Key(FcitxKey_n)));
    auto candidates = context.inputPanel().candidateList();
    assert(candidates && candidates->candidate(2).text().toString() == "你好");
    // 事件循环只能 exec 一次：300 ms 时换序帧早到了（插件约每 80 ms Poll 一次，第一条就回换序帧），
    // 断言后销毁上下文，再跑一拍让排队的 CloseSession 发出去。
    int fired = 0;
    // 精度给 1 ms：缺省的 250 ms 会让这一拍晚到，300 ms 内的 Poll 次数就数不准。
    auto timer = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 300000, 1000,
        [&](fcitx::EventSourceTime *source, uint64_t) {
            if (++fired == 1) {
                auto list = owned->inputPanel().candidateList();
                assert(list && list->candidate(0).text().toString() == "你好");
                owned.reset();
                source->setNextInterval(30000);
                source->setOneShot();
                return true;
            }
            instance.eventLoop().exit();
            return false;
        });
    instance.eventLoop().exec();
    assert(fired == 2);
    mock.join();
    size_t polls = 0, acknowledged = 0;
    for (const auto &message : mock.messages) {
        if (message.contains("Poll")) ++polls;
        if (message.contains("DisplayAcknowledged")) ++acknowledged;
    }
    assert(polls >= 2);
    // 按键那帧回报一次、换序那帧回报一次；同版本的 Poll 不重画也不重复回报。
    assert(acknowledged == 2);
}
