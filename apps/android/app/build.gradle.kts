plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

// 仓库根（ime/）：Rust workspace 与随包数据都从这里取。
val repoRoot = rootDir.resolve("../..").canonicalFile
val nativeLibs = layout.buildDirectory.dir("native/jniLibs")
val bundledAssets = layout.buildDirectory.dir("bundled/assets")

// 版本号取 core/Cargo.toml；versionCode = 主*10000 + 次*100 + 修订，-dev 与正式版同号
val appVersion = rootDir.resolve("core/Cargo.toml").readLines()
    .first { it.startsWith("version = ") }
    .substringAfter('"').substringBefore('"')
val appVersionCode = appVersion.substringBefore('-').split('.').map(String::toInt)
    .let { (major, minor, patch) -> major * 10000 + minor * 100 + patch }

// 发版 CI 从 Secrets 注入，见 docs/notes/release.md
val releaseKeystore = System.getenv("QINGJIAN_ANDROID_KEYSTORE")?.takeIf { it.isNotEmpty() }

android {
    namespace = "app.qingjian.inputmethod"
    compileSdk = 36
    ndkVersion = "29.0.13599879"

    defaultConfig {
        applicationId = "app.qingjian.inputmethod"
        minSdk = 26
        targetSdk = 36
        versionCode = appVersionCode
        versionName = appVersion
        ndk { abiFilters += "arm64-v8a" }
    }

    if (releaseKeystore != null) {
        signingConfigs {
            create("release") {
                storeFile = file(releaseKeystore)
                storePassword = System.getenv("QINGJIAN_ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("QINGJIAN_ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("QINGJIAN_ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildFeatures {
        buildConfig = true
    }

    buildTypes {
        debug {
            // 手机上装着发版签名的包时并排装调试版（-PsideBySide），不用卸载、不丢数据
            if (project.hasProperty("sideBySide")) applicationIdSuffix = ".dev"
        }
        release {
            isMinifyEnabled = false
            if (releaseKeystore != null) signingConfig = signingConfigs.getByName("release")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets["main"].apply {
        jniLibs.srcDir(nativeLibs)
        assets.srcDir(bundledAssets)
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

// Rust 侧：cargo ndk 编出 libqingjian_android.so，放进 jniLibs 源目录。要先 `cargo install cargo-ndk`。
val cargoNdk by tasks.registering(Exec::class) {
    group = "qingjian"
    description = "cargo ndk 编译 qingjian-android（arm64-v8a，release）"
    workingDir = repoRoot
    val ndkDir = android.sdkDirectory.resolve("ndk/${android.ndkVersion}")
    environment("ANDROID_NDK_HOME", ndkDir.absolutePath)
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "--platform", "26",
        "-o", nativeLibs.get().asFile.absolutePath,
        "build", "-p", "qingjian-android", "--release", "--locked",
    )
    // 增量交给 cargo 判断，这里每次都跑
    outputs.upToDateWhen { false }
}

// 随包数据：data/generated/ 的产品数据优先，没有就退回 assets/sample/ 样例（与 mac 的 bundle.sh 同一规矩）。
val stageData by tasks.registering(Sync::class) {
    group = "qingjian"
    description = "把随包数据放进 assets/data/"
    val generated = repoRoot.resolve("data/generated")
    val hasGenerated = generated.resolve("dict.qj").isFile
    into(bundledAssets.map { it.dir("data") })
    if (hasGenerated) {
        // 随包词库按 tools/release/dicts.lock 从词库仓库取（data-fetch.sh 解到 data/generated/）；安卓没有越南语输入，不带越南语两本。
        // 其他释义表与领域词库在设置里下载
        from(generated) { include("dict.qj", "lm.qj", "glossary-en.qj", "glossary-zh.qj", "english.qj", "dicts/idioms.qj") }
        // 五笔码表（形码方案用，与 PC 随包的同一本），放在数据目录顶层
        from(generated.resolve("schemes")) { include("wubi86.qj") }
    } else {
        from(repoRoot.resolve("assets/sample")) { include("dict.tsv", "glossary-en.tsv", "english.tsv") }
    }
    from(repoRoot.resolve("assets/emoji")) { include("emoji-zh.tsv", "emoji-en.tsv") }
    // 词汇等级表（英语 CEFR、日语 JLPT），生词本按级筛与统计页按级分用
    from(repoRoot.resolve("assets/levels")) { include("levels-*.tsv") }
}

tasks.named("preBuild") {
    dependsOn(cargoNdk, stageData)
}
