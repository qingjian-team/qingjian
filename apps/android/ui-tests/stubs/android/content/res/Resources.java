package android.content.res;

public class Resources {
  private final android.util.DisplayMetrics metrics = new android.util.DisplayMetrics();

  public Resources(float d) {
    metrics.density = d;
  }

  public android.util.DisplayMetrics getDisplayMetrics() {
    return metrics;
  }
}
