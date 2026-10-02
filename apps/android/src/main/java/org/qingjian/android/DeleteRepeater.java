package org.qingjian.android;

/** 退格长按重复器，使用抽象时钟保证边界行为可测试。 */
final class DeleteRepeater {
  interface Scheduler {
    void postDelayed(Runnable task, long delayMs);

    void removeCallbacks(Runnable task);
  }

  interface Action {
    void delete();
  }

  private final Scheduler scheduler;
  private final Action action;
  private final Runnable repeat = this::tick;
  private boolean active;

  DeleteRepeater(Scheduler scheduler, Action action) {
    this.scheduler = scheduler;
    this.action = action;
  }

  void start() {
    if (active) return;
    active = true;
    action.delete();
    scheduler.postDelayed(repeat, 450);
  }

  void stop() {
    if (!active) return;
    active = false;
    scheduler.removeCallbacks(repeat);
  }

  private void tick() {
    if (!active) return;
    action.delete();
    scheduler.postDelayed(repeat, 70);
  }

  boolean isActive() {
    return active;
  }
}
