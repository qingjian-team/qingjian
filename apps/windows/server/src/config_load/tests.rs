//! 缺文件、损坏配置与恢复必须可区分，错误类别不得包含配置正文。

use super::{failure_reason, read};

#[test]
fn missing_invalid_and_recovered_config_are_distinct() {
    let dir =
        std::env::temp_dir().join(format!("qingjian-config-diagnostic-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    assert_eq!(failure_reason(&read(&path).unwrap_err()), "missing_file");
    std::fs::write(
        &path,
        "[predict]\napi_key = \"DIAGNOSTIC_CANARY\"\nbroken = [",
    )
    .unwrap();
    assert_eq!(failure_reason(&read(&path).unwrap_err()), "parse_failed");
    std::fs::write(&path, "[offline_translation]\nenabled = true\n").unwrap();
    assert!(read(&path).unwrap().offline_translation.enabled);
    std::fs::write(&path, "[offline_translation]\nenabled = false\n").unwrap();
    assert!(!read(&path).unwrap().offline_translation.enabled);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn directory_is_a_read_error_instead_of_default_config() {
    assert!(read(&std::env::temp_dir()).is_err());
    let error = qingjian_platform::ConfigError::Read {
        path: std::env::temp_dir(),
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    };
    assert_eq!(failure_reason(&error), "read_failed");
}
