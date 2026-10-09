package org.qingjian.android;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Path;
import android.view.MotionEvent;
import android.view.VelocityTracker;
import android.view.View;
import android.view.ViewConfiguration;
import android.widget.OverScroller;
import java.util.Collections;
import java.util.List;

/** 深色键盘；候选尺寸只在数据更新时计算，拖动时只画可见候选。 */
final class KeyboardView extends View {
  interface Listener {
    void key(String value);

    void backspace();

    void space();

    void enter();

    void toggleLanguage();

    void toggleSymbols();

    void switchInputMethod();

    void candidate(int index);
  }

  private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
  private final Path iconPath = new Path();
  private final Listener listener;
  private final DeleteRepeater deleter;
  private final OverScroller scroller;
  private final float density, touchSlop;
  private final int minFlingVelocity, maxFlingVelocity;
  private VelocityTracker velocity;
  private CandidateStrip strip = new CandidateStrip(new float[0]);
  private String[] candidateLabels = new String[0];
  private List<Candidate> candidates = Collections.emptyList();
  private KeyboardLayout layout;
  private float layoutWidth = -1, offset, downOffset, downX, downY;
  private boolean layoutSymbols, english, symbols, shift, tracking;
  private boolean candidateGesture, candidateMoved, dragging;
  private String composing = "", status = "";

  KeyboardView(Context context, Listener listener) {
    super(context);
    this.listener = listener;
    density = getResources().getDisplayMetrics().density;
    ViewConfiguration config = ViewConfiguration.get(context);
    touchSlop = config.getScaledTouchSlop() / density;
    minFlingVelocity = config.getScaledMinimumFlingVelocity();
    maxFlingVelocity = config.getScaledMaximumFlingVelocity();
    scroller = new OverScroller(context);
    deleter =
        new DeleteRepeater(
            new DeleteRepeater.Scheduler() {
              public void postDelayed(Runnable r, long delay) {
                KeyboardView.this.postDelayed(r, delay);
              }

              public void removeCallbacks(Runnable r) {
                KeyboardView.this.removeCallbacks(r);
              }
            },
            listener::backspace);
    setFocusable(true);
  }

  void setState(String composing, List<Candidate> values, boolean english) {
    List<Candidate> next = values == null ? Collections.emptyList() : values;
    boolean textChanged = !this.composing.equals(composing), listChanged = candidates != next;
    if (!textChanged && !listChanged && this.english == english) return;
    if (textChanged || listChanged) {
      // 新候选到达后取消旧手势，避免松手时选中另一份列表。
      stopCandidateMotion();
      offset = 0;
    }
    this.composing = composing;
    this.english = english;
    if (listChanged) {
      candidates = next;
      float[] widths = new float[next.size()];
      candidateLabels = new String[next.size()];
      paint.setTextSize(17);
      for (int i = 0; i < next.size(); i++) {
        Candidate c = next.get(i);
        candidateLabels[i] = (i + 1) + " " + c.text;
        widths[i] = Math.max(100, Math.min(210, paint.measureText(candidateLabels[i]) + 30));
      }
      strip = new CandidateStrip(widths);
    }
    invalidate();
  }

  void setSymbols(boolean value) {
    if (symbols == value) return;
    symbols = value;
    layout = null;
    invalidate();
  }

  void setStatus(String value) {
    String next = value == null ? "" : value;
    if (!status.equals(next)) {
      status = next;
      invalidate();
    }
  }

  private float viewportWidth() {
    return getWidth() / density;
  }

  private KeyboardLayout keyLayout() {
    float width = viewportWidth();
    if (layout == null || layoutWidth != width || layoutSymbols != symbols) {
      layout = new KeyboardLayout(width, symbols);
      layoutWidth = width;
      layoutSymbols = symbols;
    }
    return layout;
  }

  @Override
  protected void onSizeChanged(int w, int h, int oldw, int oldh) {
    super.onSizeChanged(w, h, oldw, oldh);
    layout = null;
    stopCandidateMotion();
    offset = strip.clampOffset(offset, viewportWidth());
  }

  @Override
  protected void onMeasure(int w, int h) {
    setMeasuredDimension(
        MeasureSpec.getSize(w), resolveSize((int) (KeyboardLayout.HEIGHT * density), h));
  }

  @Override
  protected void onDraw(Canvas canvas) {
    canvas.save();
    canvas.scale(density, density);
    float width = viewportWidth();
    canvas.drawColor(Color.rgb(22, 25, 31));
    paint.setTextAlign(Paint.Align.LEFT);
    paint.setTextSize(18);
    paint.setColor(Color.rgb(240, 242, 245));
    canvas.save();
    canvas.clipRect(8, 0, width - 118, 52);
    canvas.drawText(symbols ? "123" : (english ? "EN" : composing), 10, 27, paint);
    if (!status.isEmpty()) {
      paint.setTextSize(11);
      paint.setColor(Color.rgb(255, 150, 100));
      canvas.drawText(status, 10, 48, paint);
    }
    canvas.restore();
    drawCandidates(canvas);
    for (KeySpec key : keyLayout().keys()) drawKey(canvas, key);
    canvas.restore();
  }

  private void drawCandidates(Canvas canvas) {
    float width = viewportWidth();
    int first = strip.firstVisible(offset, width), end = strip.visibleEndExclusive(offset, width);
    canvas.save();
    canvas.clipRect(0, 52, width, 110);
    for (int i = first; i < end; i++) {
      float left = strip.left(i) + offset, right = strip.right(i) + offset;
      Candidate c = candidates.get(i);
      paint.setColor(Color.rgb(48, 52, 60));
      canvas.drawRoundRect(left, 54, right, 108, 4, 4, paint);
      canvas.save();
      canvas.clipRect(left + 6, 54, right - 6, 108);
      paint.setTextAlign(Paint.Align.LEFT);
      paint.setTextSize(17);
      paint.setColor(Color.rgb(240, 242, 245));
      canvas.drawText(candidateLabels[i], left + 6, 75, paint);
      paint.setTextSize(12);
      paint.setColor(Color.rgb(184, 193, 207));
      canvas.drawText(c.gloss, left + 6, 97, paint);
      canvas.restore();
    }
    canvas.restore();
  }

  private void drawKey(Canvas c, KeySpec k) {
    String label = !k.function && english && !symbols ? (shift ? k.label : k.value) : k.label;
    if (k.value.equals("symbols") && symbols) label = "ABC";
    boolean active =
        k.value.equals("enter") || k.value.equals("shift") && english && !symbols && shift;
    boolean icon =
        k.value.equals("shift") || k.value.equals("backspace") || k.value.equals("enter");
    button(
        c,
        k.left,
        k.top,
        k.right - k.left,
        k.bottom - k.top,
        icon ? "" : label,
        k.function,
        active);
    if (icon) drawIcon(c, k);
  }

  private void button(
      Canvas c, float l, float t, float w, float h, String text, boolean function, boolean active) {
    paint.setColor(
        active ? Color.rgb(39, 70, 111) : function ? Color.rgb(37, 42, 50) : Color.rgb(48, 52, 60));
    c.drawRoundRect(l, t, l + w, t + h, 6, 6, paint);
    if (text.isEmpty()) return;
    paint.setColor(Color.rgb(240, 242, 245));
    paint.setTextAlign(Paint.Align.CENTER);
    paint.setTextSize(function ? 14 : 20);
    c.drawText(text, l + w / 2, t + h / 2 - (paint.ascent() + paint.descent()) / 2, paint);
  }

  private void drawIcon(Canvas c, KeySpec k) {
    paint.setStyle(Paint.Style.STROKE);
    paint.setStrokeWidth(2);
    paint.setStrokeJoin(Paint.Join.ROUND);
    paint.setColor(Color.rgb(240, 242, 245));
    float x = (k.left + k.right) / 2, y = (k.top + k.bottom) / 2;
    Path p = iconPath;
    p.reset();
    if (k.value.equals("shift")) {
      p.moveTo(x, y - 12);
      p.lineTo(x + 12, y);
      p.lineTo(x + 5, y);
      p.lineTo(x + 5, y + 12);
      p.lineTo(x - 5, y + 12);
      p.lineTo(x - 5, y);
      p.lineTo(x - 12, y);
      p.close();
    } else if (k.value.equals("backspace")) {
      p.moveTo(x - 12, y);
      p.lineTo(x - 4, y - 9);
      p.lineTo(x + 12, y - 9);
      p.lineTo(x + 12, y + 9);
      p.lineTo(x - 4, y + 9);
      p.close();
      p.moveTo(x, y - 4);
      p.lineTo(x + 7, y + 4);
      p.moveTo(x + 7, y - 4);
      p.lineTo(x, y + 4);
    } else {
      p.moveTo(x + 12, y - 10);
      p.lineTo(x + 12, y + 3);
      p.lineTo(x - 12, y + 3);
      p.moveTo(x - 5, y - 4);
      p.lineTo(x - 12, y + 3);
      p.lineTo(x - 5, y + 10);
    }
    c.drawPath(p, paint);
    paint.setStyle(Paint.Style.FILL);
  }

  @Override
  public boolean onTouchEvent(MotionEvent e) {
    if (e.getPointerCount() > 1 || e.getActionMasked() == MotionEvent.ACTION_CANCEL) {
      stopInteraction();
      return true;
    }
    float x = e.getX() / density, y = e.getY() / density;
    int action = e.getActionMasked();
    if (action == MotionEvent.ACTION_DOWN) {
      stopInteraction();
      tracking = true;
      downX = x;
      downY = y;
      downOffset = offset;
      candidateGesture = y >= 52 && y < 110;
      if (candidateGesture) {
        velocity = VelocityTracker.obtain();
        velocity.addMovement(e);
        if (getParent() != null) getParent().requestDisallowInterceptTouchEvent(true);
      } else {
        KeySpec k = keyLayout().hit(x, y);
        if (k != null && k.value.equals("backspace")) deleter.start();
      }
      return true;
    }
    if (!tracking) return true;
    if (candidateGesture) {
      if (velocity != null) velocity.addMovement(e);
      if (action == MotionEvent.ACTION_MOVE || action == MotionEvent.ACTION_UP)
        moveCandidates(x, y);
      if (action == MotionEvent.ACTION_UP) {
        tracking = false;
        if (dragging && velocity != null) {
          velocity.computeCurrentVelocity(1000, maxFlingVelocity);
          float speed = velocity.getXVelocity();
          if (Math.abs(speed) >= minFlingVelocity) {
            int maxX = Math.max(0, Math.round((strip.contentWidth() - viewportWidth()) * density));
            scroller.fling(Math.round(-offset * density), 0, Math.round(-speed), 0, 0, maxX, 0, 0);
            postInvalidateOnAnimation();
          }
        } else if (!candidateMoved && y >= 52 && y < 110) {
          int index = strip.candidateAt(x, offset);
          if (index >= 0) {
            performClick();
            listener.candidate(index);
          }
        }
        endCandidateGesture();
      }
      return true;
    }
    if (action == MotionEvent.ACTION_MOVE) {
      KeySpec k = keyLayout().hit(x, y);
      if (k == null || !k.value.equals("backspace")) deleter.stop();
      return true;
    }
    if (action != MotionEvent.ACTION_UP) return true;
    deleter.stop();
    tracking = false;
    KeySpec k = keyLayout().hit(x, y);
    if (k == null || keyLayout().hit(downX, downY) != k) return true;
    performClick();
    if (k.value.equals("switch")) listener.switchInputMethod();
    else if (k.value.equals("shift")) {
      shift = !shift;
      invalidate();
    } else if (k.value.equals("space")) listener.space();
    else if (k.value.equals("enter")) listener.enter();
    else if (k.value.equals("language")) listener.toggleLanguage();
    else if (k.value.equals("symbols")) listener.toggleSymbols();
    else if (!k.value.equals("backspace"))
      listener.key(english && !symbols && shift ? k.value.toUpperCase() : k.value);
    return true;
  }

  private void moveCandidates(float x, float y) {
    float dx = x - downX, dy = y - downY;
    if (Math.abs(dx) > touchSlop || Math.abs(dy) > touchSlop) candidateMoved = true;
    if (Math.abs(dx) > touchSlop) dragging = true;
    if (dragging) {
      float next = strip.clampOffset(downOffset + dx, viewportWidth());
      if (offset != next) {
        offset = next;
        postInvalidateOnAnimation();
      }
    }
  }

  @Override
  public void computeScroll() {
    if (scroller.computeScrollOffset()) {
      offset = strip.clampOffset(-scroller.getCurrX() / density, viewportWidth());
      postInvalidateOnAnimation();
    }
  }

  private void endCandidateGesture() {
    candidateGesture = false;
    candidateMoved = false;
    dragging = false;
    if (velocity != null) {
      velocity.recycle();
      velocity = null;
    }
    if (getParent() != null) getParent().requestDisallowInterceptTouchEvent(false);
  }

  private void stopCandidateMotion() {
    scroller.forceFinished(true);
    if (candidateGesture) tracking = false;
    endCandidateGesture();
  }

  @Override
  public void onWindowVisibilityChanged(int visibility) {
    super.onWindowVisibilityChanged(visibility);
    if (visibility != VISIBLE) stopInteraction();
  }

  @Override
  protected void onDetachedFromWindow() {
    stopInteraction();
    super.onDetachedFromWindow();
  }

  @Override
  public boolean performClick() {
    super.performClick();
    return true;
  }

  void stopInteraction() {
    deleter.stop();
    tracking = false;
    stopCandidateMotion();
  }
}
