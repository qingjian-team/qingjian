//! 开发者常用任务入口：`cargo xtask <命令>`（CI 不用它，走自己的 workflow）。

use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
用法：cargo xtask <命令>

  doctor   检查工具链、target 盘、产品数据与可选工具
  check    全 workspace 类型检查（--locked --all-targets）
  test     跑测试（非 Apple 平台排除 macOS crate；装了 nextest 就走 nextest）
  clippy   全 workspace clippy（-D warnings）
  fmt      rustfmt 全 workspace
  ci       本地跑一遍 CI 的 Rust 门禁（fmt --check、clippy、check、deny、test）
  run      跑 CLI：cargo xtask run <拼音>
";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(task) = args.next() else {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    };
    let rest: Vec<String> = args.collect();
    match task.as_str() {
        "doctor" => match doctor() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("体检失败：{error}");
                ExitCode::FAILURE
            }
        },
        "check" => cargo(&["check", "--locked", "--workspace", "--all-targets"]),
        "clippy" => cargo(&[
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
        "fmt" => cargo(&["fmt", "--all"]),
        "ci" => ci(),
        "test" => test(&rest),
        "run" => run_cli(&rest),
        other => {
            eprintln!("未知命令：{other}\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn cargo(args: &[&str]) -> ExitCode {
    if cargo_ok(args) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cargo_ok(args: &[&str]) -> bool {
    Command::new("cargo")
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn ci() -> ExitCode {
    let checks: &[(&str, &[&str])] = &[
        ("rustfmt", &["fmt", "--all", "--check"]),
        (
            "clippy",
            &[
                "clippy",
                "--locked",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        (
            "check",
            &["check", "--locked", "--workspace", "--all-targets"],
        ),
    ];
    for (name, args) in checks {
        println!("== {name}");
        if !cargo_ok(args) {
            eprintln!("{name} 失败（与 CI 的 Rust 门禁同一道门）");
            return ExitCode::FAILURE;
        }
    }
    if tool_ok("cargo", &["deny", "--version"]) {
        println!("== deny");
        if !cargo_ok(&["deny", "check"]) {
            return ExitCode::FAILURE;
        }
    } else {
        println!("== deny（跳过：cargo-deny 未装，cargo install cargo-deny --locked）");
    }
    println!("== test");
    test(&[])
}

fn test(extra: &[String]) -> ExitCode {
    let mut args = vec!["test".to_owned(), "--locked".to_owned()];
    if has_nextest() {
        args[0] = "nextest".to_owned();
        args.insert(1, "run".to_owned());
        println!("用 cargo-nextest（doctest 要 cargo test --doc 单跑）");
    }
    args.push("--workspace".to_owned());
    if !cfg!(target_os = "macos") {
        // macOS 的 crate 只在 Apple 机器上编得动，其余平台跳过（与 CI 的桌面矩阵一致）。
        args.extend(["--exclude".to_owned(), "qingjian-macos".to_owned()]);
    }
    args.extend_from_slice(extra);
    let status = Command::new("cargo").args(&args).status();
    match status {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("跑 cargo {} 失败：{error}", args.join(" "));
            ExitCode::FAILURE
        }
    }
}

fn has_nextest() -> bool {
    Command::new("cargo")
        .args(["nextest", "--version"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_cli(extra: &[String]) -> ExitCode {
    let mut args = vec![
        "run".to_owned(),
        "-p".to_owned(),
        "qingjian-cli".to_owned(),
        "--".to_owned(),
    ];
    args.extend_from_slice(extra);
    let status = Command::new("cargo").args(&args).status();
    match status {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("跑 cargo {} 失败：{error}", args.join(" "));
            ExitCode::FAILURE
        }
    }
}

fn doctor() -> io::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    println!("仓库：{}", clean_display(&root));

    let pinned = toolchain_channel(&root.join("rust-toolchain.toml"));
    let rustc = version_of("rustc", &["-V"]);
    println!(
        "工具链：{} / rust-toolchain.toml {}",
        rustc.as_deref().unwrap_or("缺失"),
        pinned.as_deref().unwrap_or("缺失")
    );
    if let (Some(pinned), Some(rustc)) = (&pinned, &rustc) {
        if !rustc.contains(pinned) {
            println!("  ! rustc 与 rust-toolchain.toml 不一致，进仓库目录后 rustup 会按它切换");
        }
    }

    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    println!("target：{}", clean_display(&target));
    if let Some(drive) = windows_drive(&target) {
        if drive.eq_ignore_ascii_case("C:") {
            println!(
                "  ! target 落在系统盘 C:，构建会把它写满；见 docs/contributing.md 的「构建环境」"
            );
        }
    }

    let generated = root.join("data/generated");
    let count = std::fs::read_dir(&generated)
        .map(|dir| dir.flatten().count())
        .unwrap_or(0);
    if count == 0 {
        println!("产品数据：data/generated 为空，跑 tools/release/data-fetch.sh（Git Bash）");
    } else {
        println!("产品数据：data/generated 有 {count} 项");
    }
    if !root.join("data/models/hanzhang-tongbian").exists() {
        println!("  - data/models/hanzhang-tongbian 未拉取（只有打包与评测要）");
    }

    for (name, args, hint) in [
        (
            "cargo-nextest",
            ["nextest", "--version"],
            "cargo install cargo-nextest --locked",
        ),
        (
            "cargo-deny",
            ["deny", "--version"],
            "cargo install cargo-deny --locked",
        ),
        (
            "cargo-bloat",
            ["bloat", "--version"],
            "cargo install cargo-bloat --locked",
        ),
        (
            "cargo-machete",
            ["machete", "--version"],
            "cargo install cargo-machete --locked",
        ),
        (
            "cargo-udeps",
            ["udeps", "--version"],
            "cargo install cargo-udeps --locked --git https://github.com/est31/cargo-udeps",
        ),
    ] {
        if tool_ok("cargo", &args) {
            println!("  OK   {name}");
        } else {
            println!("  -    {name}（{hint}）");
        }
    }

    packaging_tools(&root);
    Ok(())
}

#[cfg(windows)]
fn packaging_tools(_root: &Path) {
    let iscc = env::var_os("QINGJIAN_ISCC")
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| find_in_program_files("Inno Setup 7", "ISCC.exe"))
        .or_else(|| find_in_program_files("Inno Setup 6", "ISCC.exe"))
        .or_else(|| which("ISCC.exe"));
    match iscc {
        Some(path) => println!("  OK   打包：{}", path.display()),
        None => println!("  -    打包要 Inno Setup 7 的 ISCC.exe（装包或设 QINGJIAN_ISCC）"),
    }
    match which("signtool.exe").or_else(find_signtool) {
        Some(path) => println!("  OK   签名：{}", path.display()),
        None => println!(
            "  -    签名要 Windows SDK 的 signtool.exe（本机自签见 apps/windows/installer/sign-local.ps1）"
        ),
    }
}

#[cfg(not(windows))]
fn packaging_tools(_root: &Path) {}

fn toolchain_channel(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("channel")?;
        let rest = rest.trim_start().strip_prefix('=')?.trim();
        Some(rest.trim_matches('"').to_owned())
    })
}

fn version_of(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn tool_ok(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn windows_drive(path: &Path) -> Option<String> {
    let text = path.to_string_lossy();
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
    let bytes = text.as_bytes();
    (bytes.len() >= 2 && bytes[1] == b':').then(|| text[..2].to_owned())
}

fn clean_display(path: &Path) -> String {
    let text = path.display().to_string();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
}

#[cfg(not(windows))]
fn windows_drive(_path: &Path) -> Option<String> {
    None
}

#[cfg(windows)]
fn find_in_program_files(sub: &str, name: &str) -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(|var| env::var_os(var))
        .map(PathBuf::from)
        .collect();
    roots.push(PathBuf::from(r"C:\Program Files"));
    roots
        .into_iter()
        .map(|root| root.join(sub).join(name))
        .find(|path| path.exists())
}

#[cfg(windows)]
fn find_signtool() -> Option<PathBuf> {
    let kits = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .map(|root| root.join(r"Windows Kits\10\bin"))?;
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(kits)
        .ok()?
        .flatten()
        .map(|entry| entry.path().join("x64").join("signtool.exe"))
        .collect();
    dirs.sort();
    dirs.into_iter().rev().find(|path| path.exists())
}

#[cfg(windows)]
fn which(name: &str) -> Option<PathBuf> {
    let output = Command::new("where").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .next()
}
