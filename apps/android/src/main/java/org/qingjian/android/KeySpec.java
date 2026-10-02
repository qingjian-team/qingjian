package org.qingjian.android;

/** 键盘几何模型中的一个按键。坐标单位为 dp。 */
final class KeySpec {
  final String label;
  final String value;
  final float left, top, right, bottom;
  final boolean function;

  KeySpec(
      String label,
      String value,
      float left,
      float top,
      float right,
      float bottom,
      boolean function) {
    this.label = label;
    this.value = value;
    this.left = left;
    this.top = top;
    this.right = right;
    this.bottom = bottom;
    this.function = function;
  }

  boolean contains(float x, float y) {
    return x >= left && x <= right && y >= top && y <= bottom;
  }
}
