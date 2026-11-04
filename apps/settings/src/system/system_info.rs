//! 打包日志时附的 `system.txt`：版本、系统、架构、区域，外加各平台排查常要问的几样
//! （Windows 的版本号与系统缩放，macOS 的机型，Linux 的发行版、桌面、会话类型与输入法框架）。
//! 取不到的项写「unknown」，不让打包失败。

use std::process::Command;

/// 整份报告，一行一项。
pub fn report() -> String {
    let mut lines = vec![
        format!("qingjian {}", env!("CARGO_PKG_VERSION")),
        format!("os {} {}", std::env::consts::OS, std::env::consts::ARCH),
    ];
    lines.extend(platform());
    lines.join("\n") + "\n"
}

/// 跑一个命令拿输出的第一段非空文本；Windows 上不弹控制台窗口。
fn output(program: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW：设置程序是窗口程序，起控制台程序会闪一下黑框
        command.creation_flags(0x0800_0000);
    }
    let output = command.output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

fn or_unknown(value: Option<String>) -> String {
    value.unwrap_or_else(|| "unknown".to_owned())
}

#[cfg(windows)]
fn platform() -> Vec<String> {
    const KEY: &str = r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    // `reg query` 的值在行尾：`    DisplayVersion    REG_SZ    24H2`
    let value = |name: &str| {
        output("reg", &["query", KEY, "/v", name])?
            .lines()
            .find_map(|line| line.split_once("REG_").map(|(_, rest)| rest.to_owned()))
            .and_then(|rest| {
                rest.split_once(char::is_whitespace)
                    .map(|(_, v)| v.trim().to_owned())
            })
    };
    vec![
        // ProductName 在 Windows 11 上仍写 Windows 10，以 build 号为准（22000 起是 11）；
        // 不用 `ver`：它按控制台代码页输出，中文系统是 GBK
        format!(
            "product {}, display version {}, build {}.{}",
            or_unknown(value("ProductName")),
            or_unknown(value("DisplayVersion")),
            or_unknown(value("CurrentBuild")),
            or_unknown(value("UBR").map(|ubr| {
                u32::from_str_radix(ubr.trim_start_matches("0x"), 16)
                    .map_or(ubr.clone(), |n| n.to_string())
            })),
        ),
        format!(
            "locale {}",
            or_unknown(
                output(
                    "reg",
                    &[
                        "query",
                        r"HKCU\Control Panel\International",
                        "/v",
                        "LocaleName"
                    ]
                )
                .and_then(|text| text.split_whitespace().last().map(str::to_owned))
            )
        ),
        format!(
            "system dpi {}",
            or_unknown(
                output(
                    "reg",
                    &[
                        "query",
                        r"HKCU\Control Panel\Desktop\WindowMetrics",
                        "/v",
                        "AppliedDPI"
                    ]
                )
                .and_then(|text| text.split_whitespace().last().map(str::to_owned))
                .and_then(|hex| u32::from_str_radix(hex.trim_start_matches("0x"), 16).ok())
                .map(|dpi| dpi.to_string())
            )
        ),
    ]
}

#[cfg(target_os = "macos")]
fn platform() -> Vec<String> {
    vec![
        format!(
            "macos {} ({})",
            or_unknown(output("sw_vers", &["-productVersion"])),
            or_unknown(output("sw_vers", &["-buildVersion"]))
        ),
        format!(
            "model {}",
            or_unknown(output("sysctl", &["-n", "hw.model"]))
        ),
        format!(
            "locale {}",
            or_unknown(output("defaults", &["read", "-g", "AppleLocale"]))
        ),
    ]
}

#[cfg(not(any(windows, target_os = "macos")))]
fn platform() -> Vec<String> {
    let distribution = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                .map(|name| name.trim_matches('"').to_owned())
        });
    let env = |name: &str| or_unknown(std::env::var(name).ok());
    vec![
        format!("distribution {}", or_unknown(distribution)),
        format!("kernel {}", or_unknown(output("uname", &["-sr"]))),
        format!("LANG {}", env("LANG")),
        format!(
            "desktop {}, session {}",
            env("XDG_CURRENT_DESKTOP"),
            env("XDG_SESSION_TYPE")
        ),
        format!(
            "GTK_IM_MODULE {}, QT_IM_MODULE {}, XMODIFIERS {}",
            env("GTK_IM_MODULE"),
            env("QT_IM_MODULE"),
            env("XMODIFIERS")
        ),
        format!(
            "fcitx5 {}, fcitx {}, ibus {}",
            or_unknown(output("fcitx5", &["--version"])),
            or_unknown(output("fcitx", &["--version"])),
            or_unknown(output("ibus", &["version"]))
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_names_the_version_and_the_system() {
        let report = report();
        println!("{report}");
        assert!(report.starts_with(&format!("qingjian {}", env!("CARGO_PKG_VERSION"))));
        assert!(report.contains(std::env::consts::OS));
    }
}
