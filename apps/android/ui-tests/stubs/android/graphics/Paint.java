package android.graphics;

public class Paint {
  public static final int ANTI_ALIAS_FLAG = 1;
  public static int measurements;
  private float size = 17;

  public enum Style {
    FILL,
    STROKE
  }

  public enum Join {
    ROUND
  }

  public enum Align {
    LEFT,
    CENTER
  }

  public Paint(int flags) {}

  public void setColor(int c) {}

  public void setTextAlign(Align a) {}

  public void setTextSize(float s) {
    size = s;
  }

  public float measureText(String s) {
    measurements++;
    return s.length() * size * .5f;
  }

  public float ascent() {
    return -size * .8f;
  }

  public float descent() {
    return size * .2f;
  }

  public void setStyle(Style s) {}

  public void setStrokeWidth(float s) {}

  public void setStrokeJoin(Join j) {}
}
