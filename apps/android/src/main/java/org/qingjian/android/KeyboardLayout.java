package org.qingjian.android;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/** 10 列单位宽、两行错位的 26 键布局；绘制与命中共享这些矩形。 */
final class KeyboardLayout {
  static final float HEIGHT = 332, CANDIDATE_BOTTOM = 110;
  private final float width, unit, keyHeight = 48, gap = 4;
  private final List<KeySpec> keys = new ArrayList<>();
  private final List<KeySpec> readOnlyKeys = Collections.unmodifiableList(keys);

  KeyboardLayout(float width) {
    this(width, false);
  }

  KeyboardLayout(float width, boolean symbols) {
    this.width = width;
    unit = (width - 11 * gap) / 10;
    String[] rows =
        symbols
            ? new String[] {"1234567890", "-/:;()$&@", ".,?!'\"%"}
            : new String[] {"qwertyuiop", "asdfghjkl", "zxcvbnm"};
    addLetters(rows[0], 0, 118, 0);
    addLetters(rows[1], 1, 170, .5f * (unit + gap));
    keys.add(new KeySpec("⇧", "shift", gap, 222, 1.5f * (unit + gap) - gap, 270, true));
    addLetters(rows[2], 2, 222, 1.5f * (unit + gap));
    keys.add(
        new KeySpec(
            "⌫", "backspace", width - 1.5f * (unit + gap) + gap, 222, width - gap, 270, true));
    float y = 278, available = width - 5 * gap, scale = available / 10, x = 0;
    x = addWeighted("符号", "symbols", x, scale, 1, y);
    x = addWeighted("中/英", "language", x, scale, 1, y);
    x = addWeighted(",", ",", x, scale, .8f, y);
    x = addWeighted("空格", "space", x, scale, 4.4f, y);
    x = addWeighted(".", ".", x, scale, .8f, y);
    addWeighted("↵", "enter", x, scale, 2, y);
    keys.add(new KeySpec("切换输入法", "switch", width - 112, 8, width - 6, 52, true));
  }

  private void addLetters(String row, int rowIndex, float top, float offset) {
    float x = offset + gap;
    for (int i = 0; i < row.length(); i++) {
      float right = x + unit;
      keys.add(
          new KeySpec(
              String.valueOf(row.charAt(i)).toUpperCase(),
              String.valueOf(row.charAt(i)),
              x,
              top,
              right,
              top + keyHeight,
              false));
      x = right + gap;
    }
  }

  private float addWeighted(
      String label, String value, float left, float scale, float weight, float top) {
    float right = left + scale * weight;
    keys.add(new KeySpec(label, value, left, top, right, HEIGHT - 6, true));
    return right + gap;
  }

  List<KeySpec> keys() {
    return readOnlyKeys;
  }

  KeySpec hit(float x, float y) {
    for (KeySpec key : keys) if (key.contains(x, y)) return key;
    return null;
  }

  float width() {
    return width;
  }
}
