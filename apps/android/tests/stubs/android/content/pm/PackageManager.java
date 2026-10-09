package android.content.pm;

public final class PackageManager {
  private final String version;

  public PackageManager(String version) {
    this.version = version;
  }

  public PackageInfo getPackageInfo(String name, int flags) {
    PackageInfo info = new PackageInfo();
    info.versionName = version;
    info.versionCode = Integer.parseInt(version);
    return info;
  }
}
