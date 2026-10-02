package org.qingjian.android;

import android.graphics.Color;
import android.inputmethodservice.InputMethodService;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.view.View;
import android.view.WindowInsets;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import android.widget.FrameLayout;
import java.io.File;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.RejectedExecutionException;
import org.json.JSONArray;
import org.json.JSONObject;

/** 简词键盘输入法服务：事件在主线程，数据复制、JNI 初始化和查询在单线程后台。 */
public final class QingjianInputMethodService extends InputMethodService
    implements KeyboardView.Listener {
  private final ExecutorService executor = Executors.newSingleThreadExecutor();
  private final Handler main = new Handler(Looper.getMainLooper());
  private KeyboardView keyboard;
  private InputConnection connection;
  private volatile long engine;
  private volatile boolean engineReady;
  private volatile boolean shuttingDown;
  private volatile int requestSerial;
  private String composing = "";
  private boolean english;
  private boolean symbols;
  private boolean restricted;
  private String status = "";
  private EditorInfo editorInfo;
  private List<Candidate> candidates = Collections.emptyList();

  @Override
  public void onCreate() {
    super.onCreate();
    executor.execute(
        () -> {
          boolean dataOk = DataInstaller.install(this);
          if (!dataOk || !NativeEngine.available() || shuttingDown) {
            postStatus(dataOk ? "词库加载失败，可原样输入" : "数据复制失败，可原样输入");
            return;
          }
          long handle =
              NativeEngine.create(
                  new File(getFilesDir(), "data").getPath(),
                  new File(getFilesDir(), "user").getPath());
          if (shuttingDown) {
            if (handle != 0) NativeEngine.destroy(handle);
            return;
          }
          engine = handle;
          engineReady = handle != 0;
          postStatus(engineReady ? "" : "词库加载失败，可原样输入");
          main.post(
              () -> {
                if (!composing.isEmpty()) query();
              });
        });
  }

  @Override
  public View onCreateInputView() {
    keyboard = new KeyboardView(this, this);
    FrameLayout root = new FrameLayout(this);
    root.setBackgroundColor(Color.rgb(22, 25, 31));
    root.addView(keyboard, new FrameLayout.LayoutParams(-1, -2));
    // 安全区放在容器外沿，增加窗口高度，不压缩最后一排按键。
    root.setOnApplyWindowInsetsListener(
        (view, insets) -> {
          int left, right, bottom;
          if (Build.VERSION.SDK_INT >= 30) {
            android.graphics.Insets bars =
                insets.getInsets(
                    WindowInsets.Type.navigationBars() | WindowInsets.Type.displayCutout());
            left = bars.left;
            right = bars.right;
            bottom = bars.bottom;
          } else {
            left = insets.getSystemWindowInsetLeft();
            right = insets.getSystemWindowInsetRight();
            bottom = insets.getSystemWindowInsetBottom();
          }
          view.setPadding(left, 0, right, bottom);
          return insets;
        });
    android.view.Window window = getWindow().getWindow();
    if (window != null) {
      window.setNavigationBarColor(Color.rgb(22, 25, 31));
      View decor = window.getDecorView();
      int flags = decor.getSystemUiVisibility() & ~View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
      if (Build.VERSION.SDK_INT >= 30) window.setDecorFitsSystemWindows(false);
      else flags |= View.SYSTEM_UI_FLAG_LAYOUT_STABLE | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION;
      decor.setSystemUiVisibility(flags);
    }
    root.addOnAttachStateChangeListener(
        new View.OnAttachStateChangeListener() {
          @Override
          public void onViewAttachedToWindow(View view) {
            view.requestApplyInsets();
          }

          @Override
          public void onViewDetachedFromWindow(View view) {}
        });
    refresh();
    return root;
  }

  @Override
  public void onStartInput(EditorInfo info, boolean restarting) {
    if (keyboard != null) keyboard.stopInteraction();
    super.onStartInput(info, restarting);
    connection = getCurrentInputConnection();
    editorInfo = info;
    composing = "";
    candidates = Collections.emptyList();
    requestSerial++;
    restricted = isRestricted(info);
    int inputClass = info.inputType & InputType.TYPE_MASK_CLASS;
    symbols = inputClass == InputType.TYPE_CLASS_NUMBER || inputClass == InputType.TYPE_CLASS_PHONE;
    english = restricted;
    refresh();
  }

  @Override
  public void onStartInputView(EditorInfo info, boolean restarting) {
    super.onStartInputView(info, restarting);
    if (keyboard != null) keyboard.setSymbols(symbols);
    refresh();
  }

  @Override
  public boolean onEvaluateFullscreenMode() {
    return false;
  }

  @Override
  public void onFinishInputView(boolean finishingInput) {
    if (keyboard != null) keyboard.stopInteraction();
    super.onFinishInputView(finishingInput);
  }

  @Override
  public void onUpdateSelection(
      int oldStart, int oldEnd, int newStart, int newEnd, int candidatesStart, int candidatesEnd) {
    super.onUpdateSelection(oldStart, oldEnd, newStart, newEnd, candidatesStart, candidatesEnd);
    if (!composing.isEmpty()
        && (newStart != newEnd
            || newStart != candidatesEnd
            || candidatesStart < 0
            || candidatesEnd < candidatesStart)) {
      clearComposition();
      requestSerial++;
      refresh();
    }
  }

  @Override
  public void onFinishInput() {
    clearComposition();
    connection = null;
    editorInfo = null;
    super.onFinishInput();
  }

  @Override
  public void onDestroy() {
    shuttingDown = true;
    main.removeCallbacksAndMessages(null);
    requestSerial++;
    connection = null;
    engineReady = false;
    try {
      executor.execute(
          () -> {
            long handle = engine;
            engine = 0;
            if (handle != 0) NativeEngine.destroy(handle);
          });
    } catch (RejectedExecutionException ignored) {
    }
    executor.shutdown();
    super.onDestroy();
  }

  @Override
  public void key(String value) {
    if (symbols || english || restricted || !value.matches("[a-z]+")) {
      if (!composing.isEmpty() && !commit(composing)) return;
      commit(value);
      return;
    }
    composing += value;
    query();
  }

  @Override
  public void backspace() {
    if (!composing.isEmpty()) {
      composing = composing.substring(0, composing.length() - 1);
      query();
    } else if (connection != null) connection.deleteSurroundingTextInCodePoints(1, 0);
  }

  @Override
  public void space() {
    if (!candidates.isEmpty()) choose(0);
    else if (!composing.isEmpty()) commit(composing);
    else commit(" ");
  }

  @Override
  public void enter() {
    if (!composing.isEmpty() && !candidates.isEmpty()) {
      choose(0);
      return;
    }
    if (!composing.isEmpty()) {
      commit(composing);
      return;
    }
    if (connection == null) return;
    int action =
        editorInfo == null
            ? EditorInfo.IME_ACTION_NONE
            : editorInfo.imeOptions & EditorInfo.IME_MASK_ACTION;
    if (editorInfo != null && (editorInfo.imeOptions & EditorInfo.IME_FLAG_NO_ENTER_ACTION) != 0)
      action = EditorInfo.IME_ACTION_NONE;
    if (action != EditorInfo.IME_ACTION_NONE && action != EditorInfo.IME_ACTION_UNSPECIFIED)
      connection.performEditorAction(action);
    else commit("\n");
  }

  @Override
  public void toggleLanguage() {
    if (!composing.isEmpty()) commit(composing);
    english = !english;
    candidates = Collections.emptyList();
    requestSerial++;
    refresh();
  }

  @Override
  public void toggleSymbols() {
    if (!composing.isEmpty()) commit(composing);
    symbols = !symbols;
    if (keyboard != null) keyboard.setSymbols(symbols);
    refresh();
  }

  @Override
  public void candidate(int index) {
    if (index >= 0 && index < candidates.size()) choose(index);
  }

  /** 先保留当前拼音，再打开系统输入法选择器。 */
  @Override
  public void switchInputMethod() {
    if (!composing.isEmpty() && !commit(composing)) return;
    InputMethodManager manager = (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
    if (manager != null) manager.showInputMethodPicker();
  }

  private void query() {
    if (shuttingDown) return;
    final String pinyin = composing;
    final int serial = ++requestSerial;
    final InputConnection expected = connection;
    candidates = Collections.emptyList();
    if (expected != null) expected.setComposingText(pinyin, 1);
    if (pinyin.isEmpty() || !engineReady) {
      refresh();
      return;
    }
    final long handle = engine;
    safeExecute(
        () -> {
          List<Candidate> result;
          try {
            result =
                parse(handle == 0 ? null : NativeEngine.query(handle, pinyin), pinyin.length());
          } catch (Throwable ignored) {
            result = Collections.emptyList();
          }
          final List<Candidate> ready = result;
          if (!shuttingDown)
            main.post(
                () -> {
                  if (!shuttingDown
                      && serial == requestSerial
                      && expected == connection
                      && pinyin.equals(composing)) {
                    candidates = ready;
                    refresh();
                    annotateCandidates(ready, pinyin, serial, expected);
                  }
                });
        });
    refresh();
  }

  private void annotateCandidates(
      List<Candidate> ready, String pinyin, int serial, InputConnection expected) {
    final long handle = engine;
    safeExecute(
        () -> {
          if (shuttingDown || serial != requestSerial) return;
          List<Candidate> annotated;
          try {
            annotated = parse(NativeEngine.annotate(handle, pinyin), pinyin.length());
          } catch (Throwable ignored) {
            return;
          }
          main.post(
              () -> {
                if (shuttingDown
                    || serial != requestSerial
                    || expected != connection
                    || !pinyin.equals(composing)
                    || candidates != ready
                    || ready.size() != annotated.size()) return;
                for (int i = 0; i < ready.size(); i++) {
                  Candidate a = ready.get(i), b = annotated.get(i);
                  if (!a.text.equals(b.text) || a.consumed != b.consumed) return;
                }
                for (int i = 0; i < ready.size(); i++) ready.get(i).gloss = annotated.get(i).gloss;
                // 主候选没有变化，不重建布局或打断拖动。
                if (keyboard != null) keyboard.invalidate();
              });
        });
  }

  private void choose(int index) {
    Candidate selected = candidates.get(index);
    String original = composing;
    String tail =
        selected.consumed < original.length() ? original.substring(selected.consumed) : "";
    if (!commit(selected.text)) return;
    if (!restricted && engineReady && !shuttingDown) {
      final long handle = engine;
      final String text = selected.text;
      safeExecute(
          () -> {
            if (!shuttingDown) NativeEngine.learn(handle, text, original);
          });
    }
    composing = tail;
    candidates = Collections.emptyList();
    if (!tail.isEmpty()) query();
    else refresh();
  }

  private boolean commit(String text) {
    InputConnection current = connection;
    if (current == null) return false;
    boolean accepted = current.commitText(text, 1);
    if (accepted) {
      composing = "";
      candidates = Collections.emptyList();
      requestSerial++;
      refresh();
    }
    return accepted;
  }

  private void clearComposition() {
    if (connection != null && !composing.isEmpty()) connection.finishComposingText();
    composing = "";
    candidates = Collections.emptyList();
  }

  private void refresh() {
    if (keyboard != null) {
      keyboard.setSymbols(symbols);
      keyboard.setStatus(status);
      keyboard.setState(composing, candidates, english);
    }
  }

  private void postStatus(String value) {
    if (!shuttingDown)
      main.post(
          () -> {
            if (!shuttingDown) {
              status = value == null ? "" : value;
              refresh();
            }
          });
  }

  private void safeExecute(Runnable task) {
    if (shuttingDown) return;
    try {
      executor.execute(task);
    } catch (RejectedExecutionException ignored) {
    }
  }

  private static boolean isRestricted(EditorInfo info) {
    int type = info.inputType & InputType.TYPE_MASK_VARIATION;
    int klass = info.inputType & InputType.TYPE_MASK_CLASS;
    boolean password =
        (klass == InputType.TYPE_CLASS_TEXT
                && (type == InputType.TYPE_TEXT_VARIATION_PASSWORD
                    || type == InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD
                    || type == InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD))
            || (klass == InputType.TYPE_CLASS_NUMBER
                && type == InputType.TYPE_NUMBER_VARIATION_PASSWORD);
    return password || (info.imeOptions & EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING) != 0;
  }

  private static List<Candidate> parse(String json, int maxConsumed) {
    ArrayList<Candidate> result = new ArrayList<>();
    if (json == null) return result;
    try {
      JSONArray array = new JSONObject(json).optJSONArray("candidates");
      if (array == null) return result;
      for (int i = 0; i < array.length(); i++) {
        JSONObject item = array.optJSONObject(i);
        if (item == null) continue;
        int consumed = item.optInt("consumed", 0);
        if (consumed > 0 && consumed <= maxConsumed)
          result.add(
              new Candidate(item.optString("text", ""), item.optString("gloss", ""), consumed));
      }
    } catch (Exception ignored) {
    }
    return result;
  }
}
