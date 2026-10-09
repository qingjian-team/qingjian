package android.view;

public class View {
  public static final int VISIBLE = 0;
  private final android.content.Context context;
  private int width, height;
  public int animationInvalidations;

  public View(android.content.Context c) {
    context = c;
  }

  public android.content.res.Resources getResources() {
    return context.getResources();
  }

  public void setFocusable(boolean b) {}

  public int getWidth() {
    return width;
  }

  public int getHeight() {
    return height;
  }

  public ViewParent getParent() {
    return null;
  }

  public static class MeasureSpec {
    public static int getSize(int s) {
      return s;
    }
  }

  public static int resolveSize(int desired, int spec) {
    return Math.min(desired, spec);
  }

  protected void setMeasuredDimension(int w, int h) {
    int ow = width, oh = height;
    width = w;
    height = h;
    if (w != ow || h != oh) onSizeChanged(w, h, ow, oh);
  }

  protected void onSizeChanged(int w, int h, int ow, int oh) {}

  protected void onMeasure(int w, int h) {}

  protected void onDraw(android.graphics.Canvas c) {}

  public void invalidate() {}

  public void postInvalidateOnAnimation() {
    animationInvalidations++;
  }

  public boolean postDelayed(Runnable r, long d) {
    return true;
  }

  public boolean removeCallbacks(Runnable r) {
    return true;
  }

  public boolean onTouchEvent(MotionEvent e) {
    return false;
  }

  public void computeScroll() {}

  public void onWindowVisibilityChanged(int v) {}

  protected void onDetachedFromWindow() {}

  public boolean performClick() {
    return true;
  }
}
