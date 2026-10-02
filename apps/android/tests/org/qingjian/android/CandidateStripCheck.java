package org.qingjian.android;

import java.io.File;
import java.io.FileWriter;
import java.util.Random;

/** 候选几何缓存的固定种子随机验收。 */
public final class CandidateStripCheck {
  public static void main(String[] args) throws Exception {
    Random random = new Random(42);
    int cases = 0;
    for (int n : new int[] {0, 1, 6, 100, 1000, 10000}) {
      float[] widths = new float[n];
      for (int i = 0; i < n; i++) widths[i] = 100 + random.nextInt(111);
      float original = n == 0 ? 0 : widths[0];
      CandidateStrip strip = new CandidateStrip(widths);
      if (n > 0) widths[0] = 9999;
      if (n > 0 && strip.width(0) == 9999) fail("clone");
      for (int trial = 0; trial < 1000; trial++) {
        float offset = -random.nextFloat() * Math.max(0, strip.contentWidth() - 393);
        float x = random.nextFloat() * 393;
        int expected = linearAt(strip, x, offset);
        if (strip.candidateAt(x, offset) != expected) fail("candidateAt");
        int first = strip.firstVisible(offset, 393), end = strip.visibleEndExclusive(offset, 393);
        if (first < 0 || end < first || end - first > 6 && n <= 10000) fail("visible range");
        if (n > 0 && original <= 0) fail("width");
        cases++;
      }
    }
    CandidateStrip empty = new CandidateStrip(null);
    if (empty.size() != 0 || empty.candidateAt(0, 0) != -1 || empty.contentWidth() != 0)
      fail("empty");
    File out = new File("output/candidate-strip-check.txt");
    File parent = out.getParentFile();
    if (parent != null) parent.mkdirs();
    try (FileWriter writer = new FileWriter(out)) {
      writer.write("cases=" + cases + "\nseed=42\nmetric=二分正确性，visiblecount\nstatus=PASS\n");
    }
    System.out.println("candidate-strip: PASS cases=" + cases);
  }

  private static int linearAt(CandidateStrip strip, float x, float offset) {
    for (int i = 0; i < strip.size(); i++)
      if (x >= strip.left(i) + offset && x < strip.right(i) + offset) return i;
    return -1;
  }

  private static void fail(String reason) {
    throw new AssertionError(reason);
  }
}
