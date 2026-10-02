package android.view;

public class ViewConfiguration {
  private final float d;

  private ViewConfiguration(float d) {
    this.d = d;
  }

  public static ViewConfiguration get(android.content.Context c) {
    return new ViewConfiguration(c.getResources().getDisplayMetrics().density);
  }

  public int getScaledTouchSlop() {
    return Math.round(8 * d);
  }

  public int getScaledMinimumFlingVelocity() {
    return 50;
  }

  public int getScaledMaximumFlingVelocity() {
    return 8000;
  }
}
