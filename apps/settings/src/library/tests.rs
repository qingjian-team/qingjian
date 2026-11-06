//! 在独立进程与临时资源目录验证设置程序发现随包五笔。

use std::path::PathBuf;
use std::process::Command;

#[test]
fn installed_wubi_resources_are_discovered() {
    if let Some(root) = std::env::var_os("QINGJIAN_SETTINGS_SCHEME_PROBE") {
        let expected = PathBuf::from(root).join("assets/schemes/wubi86/table.tsv");
        let schemes = super::scan_code_schemes();
        let wubi = schemes.iter().find(|scheme| scheme.id == "wubi86").unwrap();
        assert_eq!(wubi.table, expected);
        assert_eq!(wubi.meta.license, "Apache-2.0");
        assert!(wubi.load().is_ok());
        return;
    }
    let root = std::env::temp_dir().join(format!("qingjian-settings-wubi-{}", std::process::id()));
    let target = root.join("assets/schemes/wubi86");
    std::fs::create_dir_all(&target).unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/schemes/wubi86");
    for name in ["scheme.toml", "table.tsv", "LICENSE", "README.md"] {
        std::fs::copy(source.join(name), target.join(name)).unwrap();
    }
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "library::tests::installed_wubi_resources_are_discovered",
        ])
        .env("QINGJIAN_SETTINGS_SCHEME_PROBE", &root)
        .env("QINGJIAN_RESOURCES", &root)
        .env("XDG_DATA_HOME", root.join("user-data"))
        .status()
        .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert!(status.success());
}
