//! 真实插件与 Server 的五笔上屏、重码与下一段预编辑。
#include "qingjian.h"
#include "support/server.h"
#include "support/input.h"
#include <fcitx/inputcontextmanager.h>
#include <cassert>

int main() {
    Server server("scheme = \"none\"\ncode = \"wubi86\"\n",
                  "一\tggll\t100\n开\tga\t10\n甲\taaaa\t10\n乙\taaaa\t5\n");
    char program[] = "qingjian-code", disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    Input input(instance.inputContextManager());
    input.focusIn();
    auto type = [&](const std::string &text) {
        for (auto character : text)
            assert(engine.process(&input, fcitx::Key(static_cast<fcitx::KeySym>(character))));
    };
    type("ga");
    assert(input.preedit() == "ga");
    assert(input.inputPanel().candidateList());
    type(" ");
    assert(input.committed == "开" && input.preedit().empty());
    type("ggll");
    assert(input.committed == "开一" && input.preedit().empty());
    type("aaaa");
    assert(input.preedit() == "aaaa");
    assert(engine.process(&input, fcitx::Key(FcitxKey_Down)));
    type("g");
    assert(input.committed == "开一乙" && input.preedit() == "g");
    type("a ");
    assert(input.committed == "开一乙开" && input.preedit().empty());
    return 0;
}
