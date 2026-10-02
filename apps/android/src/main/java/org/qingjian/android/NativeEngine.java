package org.qingjian.android;

/** JNI 适配层；Android 壳不在此实现词库或排序。 */
final class NativeEngine {
  private static final boolean AVAILABLE;

  static {
    boolean loaded = false;
    try {
      System.loadLibrary("qingjian_android");
      loaded = true;
    } catch (UnsatisfiedLinkError ignored) {
    }
    AVAILABLE = loaded;
  }

  private NativeEngine() {}

  static native long create(String dataDir, String userDir);

  static native String query(long handle, String pinyin);

  static native String annotate(long handle, String pinyin);

  static native void learn(long handle, String text, String pinyin);

  static native void destroy(long handle);

  static boolean available() {
    return AVAILABLE;
  }
}
