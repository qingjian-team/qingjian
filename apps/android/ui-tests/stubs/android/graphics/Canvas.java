package android.graphics;

public class Canvas {
  public int candidateTexts;

  public void save() {}

  public void restore() {}

  public void scale(float x, float y) {}

  public void clipRect(float a, float b, float c, float d) {}

  public void drawColor(int c) {}

  public void drawRoundRect(float a, float b, float c, float d, float rx, float ry, Paint p) {}

  public void drawPath(Path p, Paint paint) {}

  public void drawText(String s, float x, float y, Paint p) {
    if (s.matches("[0-9]+ .*")) candidateTexts++;
  }
}
