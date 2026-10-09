package org.qingjian.android;

import android.content.Context;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;

/** 将 APK assets 可靠复制到私有目录，临时文件完成后再替换。 */
final class DataInstaller {
  private static final String[] FILES = {"data/dict.qj", "data/glossary-en.qj", "data/lm.qj"};

  private DataInstaller() {}

  static boolean install(Context context) {
    File root = new File(context.getFilesDir(), "data");
    if (!root.exists() && !root.mkdirs()) return false;
    String marker = version(context);
    File stamp = new File(root, ".version");
    if (stamp.isFile() && marker.equals(read(stamp)) && requiredFilesPresent(context)) return true;
    boolean ok = true;
    for (String name : FILES) {
      try {
        if (!copy(context, name, new File(context.getFilesDir(), name))) ok = false;
      } catch (Exception ignored) {
        if (!"data/lm.qj".equals(name)) ok = false;
      }
    }
    if (ok)
      try (FileOutputStream out = new FileOutputStream(stamp, false)) {
        out.write(marker.getBytes("UTF-8"));
        out.getFD().sync();
      } catch (Exception ignored) {
        return false;
      }
    return ok;
  }

  private static boolean copy(Context context, String asset, File target) throws Exception {
    File temp = new File(target.getPath() + ".part");
    try (InputStream in = context.getAssets().open(asset);
        FileOutputStream out = new FileOutputStream(temp, false)) {
      byte[] buffer = new byte[8192];
      int count;
      while ((count = in.read(buffer)) >= 0) if (count > 0) out.write(buffer, 0, count);
      out.getFD().sync();
    }
    try {
      Files.move(
          temp.toPath(),
          target.toPath(),
          StandardCopyOption.ATOMIC_MOVE,
          StandardCopyOption.REPLACE_EXISTING);
    } catch (java.nio.file.AtomicMoveNotSupportedException unsupported) {
      return false;
    }
    return true;
  }

  private static String version(Context context) {
    try {
      android.content.pm.PackageInfo info =
          context.getPackageManager().getPackageInfo(context.getPackageName(), 0);
      return String.valueOf(info.versionName) + ":" + info.versionCode;
    } catch (Exception ignored) {
      return "unknown";
    }
  }

  private static String read(File file) {
    try {
      byte[] data = Files.readAllBytes(file.toPath());
      return new String(data, "UTF-8");
    } catch (Exception ignored) {
      return "";
    }
  }

  private static boolean requiredFilesPresent(Context context) {
    if (new File(context.getFilesDir(), "data/dict.qj").length() == 0
        || new File(context.getFilesDir(), "data/glossary-en.qj").length() == 0) return false;
    try (InputStream ignored = context.getAssets().open("data/lm.qj")) {
      return new File(context.getFilesDir(), "data/lm.qj").length() > 0;
    } catch (Exception optional) {
      return true;
    }
  }
}
