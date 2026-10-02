package android.view;

public class VelocityTracker {
  private float x, previousX, speed;
  private long t, previousT;

  public static VelocityTracker obtain() {
    return new VelocityTracker();
  }

  public void addMovement(MotionEvent e) {
    previousX = x;
    previousT = t;
    x = e.getX();
    t = e.time;
  }

  public void computeCurrentVelocity(int units, int max) {
    speed = t > previousT ? (x - previousX) * units / (t - previousT) : 0;
    speed = Math.max(-max, Math.min(max, speed));
  }

  public float getXVelocity() {
    return speed;
  }

  public void recycle() {}
}
