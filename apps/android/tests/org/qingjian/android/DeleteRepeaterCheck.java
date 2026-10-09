package org.qingjian.android;

import java.util.ArrayList;
import java.util.List;

/** 退格重复器虚拟时钟测试。 */
public final class DeleteRepeaterCheck {
  public static void main(String[] args) {
    FakeClock clock = new FakeClock();
    int[] count = {0};
    DeleteRepeater repeater = new DeleteRepeater(clock, () -> count[0]++);
    repeater.start();
    check(count[0] == 1, "down");
    repeater.start();
    check(count[0] == 1, "duplicate down ignored");
    clock.advance(449);
    check(count[0] == 1, "early");
    clock.advance(1);
    check(count[0] == 2, "repeat");
    clock.advance(140);
    check(count[0] == 4, "repeat cadence");
    repeater.stop();
    clock.advance(1000);
    check(count[0] == 4, "cancel stop");
    check(clock.tasks.isEmpty(), "no pending tasks after cancel");
    repeater.start();
    check(count[0] == 5, "new down");
    clock.advance(200);
    repeater.stop();
    clock.advance(1000);
    check(count[0] == 5, "short press deletes exactly once");
    repeater.start();
    clock.advance(449);
    check(count[0] == 6, "new hold has fresh delay");
    clock.advance(1);
    check(count[0] == 7, "new hold repeats once");
    repeater.stop();
    clock.advance(1000);
    check(count[0] == 7, "no leftover repeat");
    System.out.println("delete-repeater: PASS");
  }

  private static void check(boolean condition, String name) {
    if (!condition) throw new AssertionError(name);
  }

  private static final class FakeClock implements DeleteRepeater.Scheduler {
    private final List<Task> tasks = new ArrayList<>();
    long now;

    public void postDelayed(Runnable task, long delay) {
      tasks.add(new Task(task, now + delay));
    }

    public void removeCallbacks(Runnable task) {
      tasks.removeIf(item -> item.task == task);
    }

    void advance(long ms) {
      long end = now + ms;
      while (true) {
        Task next = null;
        for (Task item : tasks)
          if (item.at <= end && (next == null || item.at < next.at)) next = item;
        if (next == null) break;
        tasks.remove(next);
        now = next.at;
        next.task.run();
      }
      now = end;
    }

    private static final class Task {
      final Runnable task;
      final long at;

      Task(Runnable task, long at) {
        this.task = task;
        this.at = at;
      }
    }
  }
}
