package android.graphics;

public class Color {
  public static int rgb(int r, int g, int b) {
    return (255 << 24) | (r << 16) | (g << 8) | b;
  }
}
