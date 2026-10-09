package org.qingjian.android;

/** UI JVM smoke check for production geometry/cache and delete state machine. */
public final class UiGeometryCheck {
  public static void main(String[] args) {
    CandidateStrip strip = new CandidateStrip(new float[] {100, 120, 180, 100, 100, 100});
    if (strip.candidateAt(strip.left(2) + 1, 0) != 2 || strip.candidateAt(strip.right(2), 0) != -1)
      throw new AssertionError("candidate gap/boundary");
    if (strip.visibleEndExclusive(0, 393) > 6) throw new AssertionError("visible count");
    final int[] deletes = {0};
    DeleteRepeater.Scheduler scheduler =
        new DeleteRepeater.Scheduler() {
          public void postDelayed(Runnable task, long delay) {}

          public void removeCallbacks(Runnable task) {}
        };
    DeleteRepeater repeater = new DeleteRepeater(scheduler, () -> deletes[0]++);
    repeater.start();
    repeater.stop();
    if (deletes[0] != 1) throw new AssertionError("short delete");
    System.out.println("ui-geometry: PASS");
  }
}
