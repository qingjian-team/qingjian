package org.qingjian.android;

/** 无 Android 依赖的几何回归检查：javac 后用 java 运行。 */
public final class KeyboardLayoutCheck {
  public static void main(String[] args) {
    for (float width : new float[] {320, 360, 393, 432, 600}) check(width);
    System.out.println("keyboard-layout: PASS");
  }

  private static void check(float width) {
    KeyboardLayout layout = new KeyboardLayout(width);
    int letters = 0;
    float letterW = -1, letterH = -1;
    for (KeySpec a : layout.keys()) {
      if (!a.function) {
        letters++;
        if (letterW < 0) {
          letterW = a.right - a.left;
          letterH = a.bottom - a.top;
        }
        if (Math.abs(letterW - (a.right - a.left)) > .01
            || Math.abs(letterH - (a.bottom - a.top)) > .01) fail(width, "letter size");
      }
      if (a.left < 0 || a.right > width || a.top < 0 || a.bottom > KeyboardLayout.HEIGHT)
        fail(width, "bounds");
      for (KeySpec b : layout.keys()) if (a != b && overlap(a, b)) fail(width, "overlap");
      if (!a.function && layout.hit((a.left + a.right) / 2, (a.top + a.bottom) / 2) != a)
        fail(width, "center hit");
    }
    if (letters != 26) fail(width, "letter count");
  }

  private static boolean overlap(KeySpec a, KeySpec b) {
    return Math.min(a.right, b.right) - Math.max(a.left, b.left) > .01
        && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > .01;
  }

  private static void fail(float width, String reason) {
    throw new AssertionError(width + "dp: " + reason);
  }
}
