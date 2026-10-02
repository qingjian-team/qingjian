package android.view;

public class MotionEvent {
  public static final int ACTION_DOWN = 0, ACTION_UP = 1, ACTION_MOVE = 2, ACTION_CANCEL = 3;
  public final long time;
  private final int action, pointers;
  private final float x, y;

  public MotionEvent(int a, float x, float y, long t, int p) {
    action = a;
    this.x = x;
    this.y = y;
    time = t;
    pointers = p;
  }

  public int getActionMasked() {
    return action;
  }

  public int getPointerCount() {
    return pointers;
  }

  public float getX() {
    return x;
  }

  public float getY() {
    return y;
  }
}
