//! 当前页使用 CommonCandidateList；UI 翻页按钮回 Server 获取下一页。
#pragma once
#include <fcitx/candidatelist.h>
#include <functional>
namespace qingjian {
class List final : public fcitx::CommonCandidateList {
public:
    List(int page, int pages, std::function<void(bool)> turn) : page_(page), pages_(pages), turn_(std::move(turn)) {}
    bool hasPrev() const override { return page_ > 0; }
    bool hasNext() const override { return page_ + 1 < pages_; }
    int totalPages() const override { return pages_; }
    int currentPage() const override { return page_; }
    void prev() override { if (hasPrev()) turn_(false); }
    void next() override { if (hasNext()) turn_(true); }
private:
    /// Server 当前页。
    int page_;
    /// Server 总页数。
    int pages_;
    /// 翻页请求。
    std::function<void(bool)> turn_;
};
}
