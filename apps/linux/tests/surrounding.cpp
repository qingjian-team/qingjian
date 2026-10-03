//! 前后文截取的 Unicode 光标、选区和长度边界。
#include "../surrounding.h"
#include <cassert>
#include <string>

int main() {
    const auto selected = qingjian::surrounding("你好😀选中字世界", 3, 6);
    assert(selected["before"] == "你好😀");
    assert(selected["after"] == "世界");
    const auto reversed = qingjian::surrounding("你好😀选中字世界", 6, 3);
    assert(reversed == selected);
    assert(qingjian::selected("你好😀选中字世界", 3, 6) == "选中字");
    assert(qingjian::selected("你好😀选中字世界", 6, 3) == "选中字");
    assert(qingjian::selected("你好", 3, 0).empty());
    assert(qingjian::selected(std::string(500, 'a'), 0, 500) == std::string(500, 'a'));
    assert(qingjian::selected(std::string(501, 'a'), 0, 501).empty());
    assert(qingjian::surrounding("你好", 3, 3).is_null());
    assert(qingjian::surrounding("", 0, 0)["before"] == "");
    const std::string many(100, 'a');
    assert(qingjian::surrounding(many, 90, 90)["before"] == std::string(80, 'a'));
    assert(qingjian::surrounding(many, 0, 0)["after"] == std::string(80, 'a'));
}
