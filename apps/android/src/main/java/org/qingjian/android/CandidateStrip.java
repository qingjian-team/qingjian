package org.qingjian.android;

/** 候选卡片的纯 Java 几何缓存，坐标单位为 dp。 */
final class CandidateStrip {
  private static final float MARGIN = 10f, GAP = 5f;
  private final float[] lefts, rights;
  private final float contentWidth;

  CandidateStrip(float[] widths) {
    int n = widths == null ? 0 : widths.length;
    lefts = new float[n];
    rights = new float[n];
    float x = MARGIN;
    for (int i = 0; i < n; i++) {
      float width = Math.max(0, widths[i]);
      lefts[i] = x;
      rights[i] = x + width;
      x = rights[i] + GAP;
    }
    contentWidth = n == 0 ? 0 : x - GAP + MARGIN;
  }

  int size() {
    return lefts.length;
  }

  float left(int index) {
    return lefts[index];
  }

  float right(int index) {
    return rights[index];
  }

  float width(int index) {
    return rights[index] - lefts[index];
  }

  float contentWidth() {
    return contentWidth;
  }

  float clampOffset(float offset, float viewportWidth) {
    return Math.max(Math.min(0, viewportWidth - contentWidth), Math.min(0, offset));
  }

  int candidateAt(float x, float offset) {
    int lo = 0, hi = size() - 1;
    while (lo <= hi) {
      int mid = (lo + hi) >>> 1;
      float screenLeft = lefts[mid] + offset, screenRight = rights[mid] + offset;
      if (x < screenLeft) hi = mid - 1;
      else if (x >= screenRight) lo = mid + 1;
      else return mid;
    }
    return -1;
  }

  int firstVisible(float offset, float viewportWidth) {
    float start = -offset;
    int lo = 0, hi = size();
    while (lo < hi) {
      int mid = (lo + hi) >>> 1;
      if (rights[mid] <= start) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  }

  int visibleEndExclusive(float offset, float viewportWidth) {
    float end = viewportWidth - offset;
    int lo = 0, hi = size();
    while (lo < hi) {
      int mid = (lo + hi) >>> 1;
      if (lefts[mid] < end) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  }
}
