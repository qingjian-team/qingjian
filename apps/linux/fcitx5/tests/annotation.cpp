//! 真 Server 与 Fcitx5 面板显示分段假名，Alt+数字只上屏原始译文。
#include "qingjian.h"
#include "support/context.h"
#include "support/server.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>

int main() {
    Server server("learning_language = \"ja\"\n");
    server.stop();
    std::filesystem::remove(server.directory / "server.sock");
    std::filesystem::create_directories(server.directory / "assets/glossary");
    std::ofstream(server.directory / "assets/glossary/glossary-ja.tsv")
        << "开发\tv. 開発する|かいはつする\tn. 開発|かいはつ\n";
    server.start();
    char program[] = "qingjian-test"; char disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    Context context(instance.inputContextManager());
    context.focusIn();
    auto compose = [&] {
        for (char c : std::string("kaifa"))
            assert(engine.process(&context, fcitx::Key(static_cast<fcitx::KeySym>(c))));
        auto list = context.inputPanel().candidateList();
        assert(list && list->candidate(0).text().toString() == "开发");
        const auto annotation = list->candidate(0).comment().toString();
        assert(annotation == "v. 開発(かいはつ)する · 生");
    };
    compose();
    assert(engine.process(&context, fcitx::Key(FcitxKey_1, fcitx::KeyState::Alt)));
    assert(context.committed == "開発する");
    assert(context.inputPanel().empty());
    context.committed.clear();
    compose();
    assert(engine.process(&context, fcitx::Key(FcitxKey_exclam, fcitx::KeyStates{fcitx::KeyState::Alt, fcitx::KeyState::Shift})));
    assert(context.committed == "開発");
    context.committed.clear();
    compose();
    context.inputPanel().candidateList()->candidate(0).select(&context);
    assert(context.committed == "开发");
}
