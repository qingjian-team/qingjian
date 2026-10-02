package org.qingjian.android;

import android.content.Context;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

/** 用最小 Context 假体在 JVM 验证真实复制代码的升级及失败保护。 */
public final class DataInstallerCheck {
  public static void main(String[] args) throws Exception {
    File root = Files.createTempDirectory("jianci-data-test").toFile();
    File assets = new File(root, "assets/data");
    File user = new File(root, "files");
    assets.mkdirs();
    user.mkdirs();
    String[] names = {"dict.qj", "glossary-en.qj", "lm.qj"};
    for (String name : names)
      Files.write(
          new File(assets, name).toPath(), ("old-" + name).getBytes(StandardCharsets.UTF_8));
    Context context = new Context(user, new File(root, "assets"), "1");
    if (!DataInstaller.install(context)) throw new AssertionError("first install");
    for (String name : names)
      Files.write(
          new File(assets, name).toPath(), ("new-" + name).getBytes(StandardCharsets.UTF_8));
    context = new Context(user, new File(root, "assets"), "2");
    if (!DataInstaller.install(context)) throw new AssertionError("upgrade install");
    for (String name : names) {
      String content =
          new String(
              Files.readAllBytes(new File(user, "data/" + name).toPath()), StandardCharsets.UTF_8);
      if (!content.equals("new-" + name)) throw new AssertionError("stale data: " + name);
    }
    Files.delete(new File(assets, "dict.qj").toPath());
    context = new Context(user, new File(root, "assets"), "3");
    if (DataInstaller.install(context))
      throw new AssertionError("missing required asset must fail");
    String retained =
        new String(
            Files.readAllBytes(new File(user, "data/dict.qj").toPath()), StandardCharsets.UTF_8);
    if (!retained.equals("new-dict.qj")) throw new AssertionError("old valid data lost on failure");
    System.out.println(
        "DataInstaller: initial copy, upgrade replacement, failed-copy preservation passed");
  }
}
