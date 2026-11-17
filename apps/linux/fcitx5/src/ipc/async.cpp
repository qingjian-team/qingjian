//! 后台串行执行一项事务，完成后通过 Fcitx EventDispatcher 回到主线程。
#include "async.h"
#include <stdexcept>
#include <utility>

namespace qingjian {
AsyncConnection::AsyncConnection(fcitx::EventLoop &loop) {
    dispatcher_.attach(&loop);
    worker_ = std::thread([this] { run(); });
}
AsyncConnection::~AsyncConnection() {
    *alive_ = false;
    {
        std::lock_guard lock(mutex_);
        stopping_.store(true);
        cancelled_.store(true);
    }
    ready_.notify_one();
    worker_.join();
    dispatcher_.detach();
}
void AsyncConnection::submit(Work work, Done done) {
    {
        std::lock_guard lock(mutex_);
        if (job_) throw std::logic_error("IPC transaction already queued");
        cancelled_.store(false);
        job_.emplace(std::move(work), std::move(done));
    }
    ready_.notify_one();
}
bool AsyncConnection::wait(std::chrono::steady_clock::time_point deadline) {
    {
        std::unique_lock lock(mutex_);
        if (!ready_.wait_until(lock, deadline,
                [this] { return stopping_.load() || completed_.has_value(); })
            || stopping_.load()) return false;
    }
    dispatch();
    return true;
}
void AsyncConnection::cancel() {
    cancelled_.store(true);
}
void AsyncConnection::dispatch() {
    std::optional<std::pair<Done, Reply>> completed;
    {
        std::lock_guard lock(mutex_);
        completed.swap(completed_);
    }
    if (completed) completed->first(std::move(completed->second));
}
void AsyncConnection::run() {
    Connection connection(std::chrono::seconds(5), &cancelled_);
    while (!stopping_.load()) {
        std::unique_lock lock(mutex_);
        ready_.wait(lock, [this] { return stopping_.load() || job_.has_value(); });
        if (stopping_.load()) break;
        auto [work, done] = std::move(*job_);
        job_.reset();
        lock.unlock();
        Reply reply;
        try { reply = work(connection); }
        catch (const std::exception &) { connection.close(); }
        {
            std::lock_guard lock(mutex_);
            completed_.emplace(std::move(done), std::move(reply));
        }
        ready_.notify_all();
        dispatcher_.schedule([this, alive = alive_] {
            if (*alive) dispatch();
        });
    }
}
}
