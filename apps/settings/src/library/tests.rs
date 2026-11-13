//! 在独立进程与临时资源目录验证设置程序发现随包五笔。

use std::path::PathBuf;
use std::process::Command;

#[test]
fn installed_wubi_resources_are_discovered() {
    if let Some(root) = std::env::var_os("QINGJIAN_SETTINGS_SCHEME_PROBE") {
        let expected = PathBuf::from(root).join("data/generated/schemes/wubi86.qj");
        let schemes = super::scan_code_schemes();
        let wubi = schemes.iter().find(|scheme| scheme.id == "wubi86").unwrap();
        assert_eq!(wubi.table, expected);
        assert_eq!(wubi.meta.license, "Apache-2.0");
        assert!(wubi.load().unwrap().wubi_phrases);
        return;
    }
    let root = std::env::temp_dir().join(format!("qingjian-settings-wubi-{}", std::process::id()));
    let target = root.join("data/generated/schemes");
    std::fs::create_dir_all(&target).unwrap();
    qingjian_dictionary::CodeTable::parse("一\tggll\t1\n")
        .unwrap()
        .write_qj(
            &target.join("wubi86.qj"),
            &qingjian_format::Metadata {
                license: "Apache-2.0".to_owned(),
                ..Default::default()
            },
        )
        .unwrap();
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
