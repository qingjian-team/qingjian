//! 真实插件到 Rust Server：128 活跃会话、交错组句、重入、重启和键盘来源。
#include "qingjian.h"
#include "key/mapping.h"
#include "support/server.h"
#include "support/input.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputmethodentry.h>
#include <cassert>

int main() {
    Server server("shift_letter = \"compose\"\n");
    char program[] = "qingjian-real"; char disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments); instance.initialize();
    auto engineOwner = std::make_unique<fcitx::QingjianEngine>(&instance.addonManager());
    auto &engine = *engineOwner;
    fcitx::InputMethodEntry entry("qingjian", "qingjian", "zh_CN", "qingjian");
    auto type = [&](Input &context, const std::string &text) {
        for (auto c : text) assert(engine.process(&context, fcitx::Key(static_cast<fcitx::KeySym>(c))));
    };
    std::vector<std::unique_ptr<Input>> contexts;
    for (size_t i = 0; i < 128; ++i) {
        contexts.push_back(std::make_unique<Input>(instance.inputContextManager()));
        type(*contexts.back(), i % 2 ? "hao" : "ni");
    }
    assert(server.sockets() == 2); // 一条监听 socket + 一条插件连接。
    for (size_t i = 0; i < contexts.size(); ++i) {
        assert(engine.process(contexts[i].get(), fcitx::Key(FcitxKey_space)));
        assert(contexts[i]->committed == (i % 2 ? "好" : "你"));
    }
    auto &a = *contexts[0]; auto &b = *contexts[1];
    a.committed.clear(); b.committed.clear();
    type(a, "hao6"); assert(engine.process(&a, fcitx::Key(FcitxKey_Return))); assert(a.committed == "hao6");
    a.committed.clear(); type(a, "gpt-6 "); assert(a.committed == "gpt-6 ");
    a.committed.clear(); type(a, "Ni"); assert(engine.process(&a, fcitx::Key(FcitxKey_space))); assert(a.committed == "你");
    a.committed.clear(); type(a, "ni");
    a.preeditHook = [&] { type(b, "hao"); assert(engine.process(&b, fcitx::Key(FcitxKey_space))); };
    assert(engine.process(&a, fcitx::Key(FcitxKey_space)));
    assert(a.committed == "你" && b.committed == "好");
    // 同一上下文 Reset 重入撤销旧提交，已消费按键仍返回 consumed。
    a.committed.clear(); type(a, "ni");
    a.preeditHook = [&] { fcitx::InputContextEvent event(&a, fcitx::EventType::InputContextReset); engine.reset(entry, event); };
    assert(engine.process(&a, fcitx::Key(FcitxKey_space))); assert(a.committed.empty());
    type(a, "ni"); assert(engine.process(&a, fcitx::Key(FcitxKey_space))); assert(a.committed == "你");
    // Shift 由 Server 判定；Shift+字母不切模式。
    a.committed.clear();
    fcitx::KeyEvent press(&a, fcitx::Key(FcitxKey_Shift_L), false), release(&a, fcitx::Key(FcitxKey_Shift_L), true);
    engine.keyEvent(entry, press); assert(engine.process(&a, fcitx::Key(FcitxKey_N, fcitx::KeyState::Shift))); engine.keyEvent(entry, release);
    assert(engine.process(&a, fcitx::Key(FcitxKey_Return))); assert(a.committed == "N");
    auto tapShift = [&] {
        fcitx::KeyEvent down(&a, fcitx::Key(FcitxKey_Shift_L), false), up(&a, fcitx::Key(FcitxKey_Shift_L), true);
        engine.keyEvent(entry, down); engine.keyEvent(entry, up);
    };
    a.committed.clear(); tapShift(); type(a, "ni"); assert(engine.process(&a, fcitx::Key(FcitxKey_space)));
    assert(a.committed != "你");
    type(b, "ni"); assert(engine.process(&b, fcitx::Key(FcitxKey_space))); assert(b.committed == "好你");
    tapShift(); a.committed.clear(); type(a, "ni"); assert(engine.process(&a, fcitx::Key(FcitxKey_space))); assert(a.committed == "你");
    b.committed = "好";
    // 单一上下文的显示回调可以销毁它自己，旧按键仍算消费且无悬空访问。
    auto doomed = std::make_unique<Input>(instance.inputContextManager());
    type(*doomed, "ni"); auto *doomedPointer = doomed.get();
    doomed->preeditHook = [&] { doomed.reset(); };
    assert(engine.process(doomedPointer, fcitx::Key(FcitxKey_space)));
    // 小键盘来源、运算符与 NumLock 关闭导航键。
    for (int digit = 0; digit <= 9; ++digit) {
        auto key = qingjian::mapKey(fcitx::Key(static_cast<fcitx::KeySym>(FcitxKey_KP_0 + digit)));
        assert(key["virtual_key"] == 0x60 + digit && key["character"] == std::string(1, '0' + digit));
    }
    for (auto [sym, code] : {std::pair{FcitxKey_KP_Add, 0x6b}, {FcitxKey_KP_Subtract, 0x6d}, {FcitxKey_KP_Decimal, 0x6e}, {FcitxKey_KP_Divide, 0x6f}, {FcitxKey_KP_Multiply, 0x6a}, {FcitxKey_KP_Enter, 0x0d}, {FcitxKey_KP_Home, 0x24}, {FcitxKey_KP_Left, 0x25}, {FcitxKey_KP_Delete, 0x2e}})
        assert(qingjian::mapKey(fcitx::Key(sym))["virtual_key"] == code);
    a.committed.clear();
    for (auto symbol : {FcitxKey_KP_1, FcitxKey_KP_Add, FcitxKey_KP_2, FcitxKey_KP_Subtract,
                        FcitxKey_KP_3, FcitxKey_KP_Multiply, FcitxKey_KP_4, FcitxKey_KP_Divide,
                        FcitxKey_KP_5, FcitxKey_KP_Decimal, FcitxKey_KP_0}) {
        assert(!engine.process(&a, fcitx::Key(symbol)));
        a.commitString(fcitx::Key::keySymToUTF8(symbol)); // 应用接收透传键的实际字符。
    }
    assert(a.committed == "1+2-3*4/5.0");
    type(a, "hao"); assert(engine.process(&a, fcitx::Key(FcitxKey_KP_Enter)));
    assert(a.committed == "1+2-3*4/5.0hao");
    assert(a.preedit().empty());
    // A 私密，B 普通；密码/禁用 C 不创建新的连接或污染 B。
    a.setCapabilityFlags(fcitx::CapabilityFlags(fcitx::CapabilityFlag::Preedit) | fcitx::CapabilityFlag::Sensitive);
    type(a, "privateprobe"); type(b, "ni");
    auto *disabledSession = static_cast<qingjian::Session *>(contexts[2]->property("qingjian-session"));
    const auto disabledId = disabledSession->id;
    contexts[2]->setCapabilityFlags(fcitx::CapabilityFlag::Password);
    assert(!engine.process(contexts[2].get(), fcitx::Key(FcitxKey_n)));
    assert(engine.process(&b, fcitx::Key(FcitxKey_space))); assert(b.committed == "好你");
    contexts[2]->setCapabilityFlags(fcitx::CapabilityFlag::Preedit);
    type(*contexts[2], "ni");
    assert(static_cast<qingjian::Session *>(contexts[2]->property("qingjian-session"))->id > disabledId);
    // 保留旧列表再销毁，旧回调不能提交到后来创建的上下文。
    contexts[3]->focusIn(); type(*contexts[3], "ni");
    auto old = contexts[3]->inputPanel().candidateList(); contexts[3].reset();
    auto replacement = std::make_unique<Input>(instance.inputContextManager());
    old->candidate(0).select(replacement.get()); assert(replacement->committed.empty());
    type(*replacement, "ni"); assert(engine.process(replacement.get(), fcitx::Key(FcitxKey_space)));
    for (int i = 0; i < 160; ++i) { Input transient(instance.inputContextManager()); type(transient, "ni"); }
    type(*replacement, "hao"); assert(server.sockets() == 2);
    server.stop();
    for (const auto &file : std::filesystem::recursive_directory_iterator(server.directory)) {
        if (!file.is_regular_file() || file.path().filename() == "dict.tsv") continue;
        std::ifstream input(file.path()); std::string content((std::istreambuf_iterator<char>(input)), {});
        assert(content.find("privateprobe") == std::string::npos);
    }
    assert(!engine.process(replacement.get(), fcitx::Key(FcitxKey_space)));
    assert(replacement->preedit().empty());
    server.start();
    const auto committed = replacement->committed;
    type(*replacement, "hao"); assert(engine.process(replacement.get(), fcitx::Key(FcitxKey_space)));
    assert(replacement->committed == committed + "好" && server.sockets() == 2);
    replacement->focusIn(); type(*replacement, "ni");
    auto finalCandidates = replacement->inputPanel().candidateList();
    auto shared = static_cast<qingjian::Session *>(replacement->property("qingjian-session"))->owner.lock();
    { Input transient(instance.inputContextManager()); type(transient, "ni"); }
    assert(!shared->retired.empty());
    auto timer = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 30000, 0,
        [&](fcitx::EventSourceTime *, uint64_t) { instance.eventLoop().exit(); return false; });
    instance.eventLoop().exec();
    assert(shared->retired.empty()); // 最后销毁后的空闲回收不依赖后续按键。
    timer.reset();
    const auto finalCommit = replacement->committed;
    engineOwner.reset();
    finalCandidates->candidate(0).select(replacement.get());
    finalCandidates->toPageable()->next();
    assert(replacement->committed == finalCommit);

}
