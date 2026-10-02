package android.widget;

public class OverScroller {
  public static int lastVelocity, flings;
  private int x;
  private boolean pending;

  public OverScroller(android.content.Context c) {}

  public void forceFinished(boolean finished) {
    if (finished) pending = false;
  }

  public void fling(int sx, int sy, int vx, int vy, int minx, int maxx, int miny, int maxy) {
    lastVelocity = vx;
    flings++;
    x = Math.max(minx, Math.min(maxx, sx + Integer.signum(vx) * 60));
    pending = true;
  }

  public boolean computeScrollOffset() {
    boolean p = pending;
    pending = false;
    return p;
  }

  public int getCurrX() {
    return x;
  }
}
