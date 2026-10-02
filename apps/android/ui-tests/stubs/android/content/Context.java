package android.content;

public class Context {
  private final android.content.res.Resources resources;

  public Context(float d) {
    resources = new android.content.res.Resources(d);
  }

  public android.content.res.Resources getResources() {
    return resources;
  }
}
