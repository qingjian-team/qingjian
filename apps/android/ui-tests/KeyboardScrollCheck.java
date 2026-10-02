package org.qingjian.android;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.view.MotionEvent;
import android.widget.OverScroller;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;

/** 编译真实 View 的 JVM 检查；假 Android 对象只验证逻辑和调用次数，不验证帧率。 */
public final class KeyboardScrollCheck {
  public static void main(String[] args) throws Exception {
    for (float density : new float[] {1, 3})
      for (int count : new int[] {32, 100, 1000}) check(density, count);
    System.out.println("keyboard-scroll: PASS density=1,3 candidates=32,100,1000");
    System.out.println(
        "MOVE follows finger; no selection after drag; bounded visible draws; zero per-frame"
            + " measureText; layout reused");
    System.out.println(
        "Fake Android tests do not measure actual Android fling physics or device frame timing.");
  }

  private static void check(float d, int count) throws Exception {
    Recorder calls = new Recorder();
    KeyboardView view = new KeyboardView(new Context(d), calls);
    view.onMeasure(Math.round(393 * d), Math.round(332 * d));
    List<Candidate> list = new ArrayList<>();
    for (int i = 0; i < count; i++) list.add(new Candidate("词" + i, "word" + i, 1));
    view.setState("ci", list, false);
    int measurements = Paint.measurements;
    Canvas canvas = new Canvas();
    view.onDraw(canvas);
    Object layout = field(view, "layout");
    int maxVisible = 0;
    for (int frame = 0; frame < 10; frame++) {
      canvas = new Canvas();
      view.onDraw(canvas);
      maxVisible = Math.max(maxVisible, canvas.candidateTexts);
      require(canvas.candidateTexts <= 5, "only visible candidates rendered");
    }
    require(Paint.measurements == measurements, "no measurements during repeated draw");
    require(field(view, "layout") == layout, "key layout retained across frames");
    view.setState("ci", list, false);
    require(Paint.measurements == measurements, "unchanged list does not rebuild cache");
    event(view, d, 0, 300, 75, 0, 1);
    event(view, d, 2, 200, 75, 20, 1);
    near(offset(view), -100, "MOVE updates before UP");
    int scheduled = view.animationInvalidations;
    require(scheduled > 0, "MOVE schedules frame");
    event(view, d, 2, 100, 75, 40, 1);
    near(offset(view), -200, "second MOVE follows finger");
    Object cachedStrip = field(view, "strip");
    for (Candidate c : list) c.gloss = "updated English annotation";
    view.invalidate();
    view.onDraw(new Canvas());
    view.setState("ci", list, false);
    near(offset(view), -200, "annotation preserves ongoing drag");
    require(field(view, "strip") == cachedStrip, "annotation preserves candidate layout");
    require(Paint.measurements == measurements, "annotation does not measure text again");
    event(view, d, 1, 95, 75, 60, 1);
    require(calls.candidates == 0, "drag cannot pick candidate");
    require(OverScroller.lastVelocity > 0, "left fling uses positive scroll coordinate");
    view.computeScroll();
    require(offset(view) < -205, "fling tick advances offset");
    float prior = offset(view);
    event(view, d, 0, 30, 75, 100, 1);
    view.computeScroll();
    near(offset(view), prior, "new DOWN stops previous fling");
    event(view, d, 2, 390, 75, 200, 1);
    near(offset(view), 0, "right drag clamps at start");
    event(view, d, 1, 390, 75, 2000, 1);
    require(calls.candidates == 0, "return drag never picks candidate");
    event(view, d, 0, 20, 75, 2100, 1);
    event(view, d, 1, 20, 75, 2120, 1);
    require(calls.index == 0, "tap uses cached card index");
    int selected = calls.candidates;
    event(view, d, 0, 300, 75, 2200, 1);
    event(view, d, 2, -1000000, 75, 2300, 1);
    CandidateStrip strip = (CandidateStrip) field(view, "strip");
    near(offset(view), strip.clampOffset(-1000000, 393), "end boundary");
    event(view, d, 3, 0, 75, 2310, 1);
    event(view, d, 1, 20, 75, 2320, 1);
    require(calls.candidates == selected, "cancel suppresses release");
    event(view, d, 0, 20, 75, 2400, 1);
    event(view, d, 2, 20, 75, 2410, 2);
    event(view, d, 1, 20, 75, 2420, 1);
    require(calls.candidates == selected, "multi-pointer cancellation");
    event(view, d, 0, 20, 75, 2500, 1);
    view.setState("new", new ArrayList<>(list), false);
    event(view, d, 1, 20, 75, 2520, 1);
    require(calls.candidates == selected, "replacement cancels stale gesture");
    event(view, d, 0, 20, 75, 2600, 1);
    view.onWindowVisibilityChanged(8);
    event(view, d, 1, 20, 75, 2620, 1);
    require(calls.candidates == selected, "hidden window stops gesture");
    event(view, d, 0, 366, 245, 2700, 1);
    require(calls.deletes == 1, "delete on DOWN");
    event(view, d, 1, 366, 245, 2720, 1);
    require(calls.deletes == 1, "no extra delete on UP");
    event(view, d, 0, 15, 140, 2800, 1);
    event(view, d, 1, 15, 140, 2820, 1);
    require("q".equals(calls.key), "letter still uses correct key");
    event(view, d, 0, 18, 300, 2900, 1);
    event(view, d, 1, 18, 300, 2920, 1);
    require(calls.symbols == 1, "symbols button still works");
    event(view, d, 0, 330, 30, 3000, 1);
    event(view, d, 1, 330, 30, 3020, 1);
    require(calls.switches == 1, "switch button still works");
    System.out.println(
        "count="
            + count
            + " density="
            + d
            + " baselineCandidateDraws="
            + count
            + " maxVisibleDraws="
            + maxVisible
            + " perFrameMeasurements=0");
  }

  private static Object field(KeyboardView view, String name) throws Exception {
    Field f = KeyboardView.class.getDeclaredField(name);
    f.setAccessible(true);
    return f.get(view);
  }

  private static float offset(KeyboardView v) throws Exception {
    return (Float) field(v, "offset");
  }

  private static void event(
      KeyboardView v, float d, int a, float x, float y, long t, int pointers) {
    v.onTouchEvent(new MotionEvent(a, x * d, y * d, t, pointers));
  }

  private static void require(boolean c, String reason) {
    if (!c) throw new AssertionError(reason);
  }

  private static void near(float actual, float expected, String reason) {
    require(
        Math.abs(actual - expected) < .1, reason + " actual=" + actual + " expected=" + expected);
  }

  private static final class Recorder implements KeyboardView.Listener {
    int candidates, index = -1, deletes, symbols, switches;
    String key;

    public void key(String value) {
      key = value;
    }

    public void backspace() {
      deletes++;
    }

    public void space() {}

    public void enter() {}

    public void toggleLanguage() {}

    public void toggleSymbols() {
      symbols++;
    }

    public void switchInputMethod() {
      switches++;
    }

    public void candidate(int i) {
      candidates++;
      index = i;
    }
  }
}
