package android.content;

import android.content.pm.PackageManager;
import android.content.res.AssetManager;
import java.io.File;

/** 仅用于 DataInstaller JVM 检查，不进入 APK。 */
public final class Context {
  private final File files;
  private final AssetManager assets;
  private final PackageManager packages;

  public Context(File files, File assets, String version) {
    this.files = files;
    this.assets = new AssetManager(assets);
    this.packages = new PackageManager(version);
  }

  public File getFilesDir() {
    return files;
  }

  public AssetManager getAssets() {
    return assets;
  }

  public PackageManager getPackageManager() {
    return packages;
  }

  public String getPackageName() {
    return "org.qingjian.android";
  }
}
