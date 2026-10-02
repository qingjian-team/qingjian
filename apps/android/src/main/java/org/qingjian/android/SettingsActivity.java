package org.qingjian.android;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.provider.Settings;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

/** 输入法设置入口，使用系统页面完成启用与切换。 */
public final class SettingsActivity extends Activity {
  @Override
  public void onCreate(Bundle state) {
    super.onCreate(state);
    LinearLayout page = new LinearLayout(this);
    page.setOrientation(LinearLayout.VERTICAL);
    page.setPadding(32, 36, 32, 24);
    TextView title = new TextView(this);
    title.setText("简词键盘");
    title.setTextSize(27);
    page.addView(title, new LinearLayout.LayoutParams(-1, -2));
    TextView intro = new TextView(this);
    intro.setText(
        "离线实验版  ·  26 键拼音\n"
            + "输入中文时显示英语释义，帮助逐步积累词汇。数据只在本机处理；密码框和无痕输入不会学习。\n\n"
            + "本项目为非官方实验版，不使用上游 logo。词库与释义数据按各自许可证分发；代码 GPL 来源署名见项目 LICENSE。");
    intro.setTextSize(16);
    intro.setPadding(0, 20, 0, 24);
    page.addView(intro, new LinearLayout.LayoutParams(-1, -2));
    Button enable = new Button(this);
    enable.setText("启用简词键盘");
    enable.setOnClickListener(
        v -> startActivity(new Intent(Settings.ACTION_INPUT_METHOD_SETTINGS)));
    page.addView(enable, new LinearLayout.LayoutParams(-1, -2));
    Button switchButton = new Button(this);
    switchButton.setText("切换输入法");
    switchButton.setOnClickListener(
        v -> {
          android.view.inputmethod.InputMethodManager manager =
              (android.view.inputmethod.InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
          if (manager != null) manager.showInputMethodPicker();
        });
    page.addView(switchButton, new LinearLayout.LayoutParams(-1, -2));
    TextView footer = new TextView(this);
    footer.setText("Android 8.0+ · ARM64 · 实验版；尚未在真机验证。");
    footer.setPadding(0, 28, 0, 0);
    page.addView(footer, new LinearLayout.LayoutParams(-1, -2));
    setContentView(page);
  }
}
