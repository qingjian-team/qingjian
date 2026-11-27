//! 更新状态与索引结构的约定：字段默认值、落盘与读取。

use std::path::PathBuf;

use crate::{Asset, Available, DOWNLOAD_URL, Release, Target, UpdateState};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("qingjian-update-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{name}-{}.json", std::process::id()))
}

#[test]
fn default_state_serializes_every_field() {
    assert_eq!(
        serde_json::to_value(UpdateState::default()).unwrap(),
        serde_json::json!({"checked_at": 0, "channel": null, "available": null})
    );
}

#[test]
fn missing_state_file_loads_as_default() {
    let path = scratch("missing");
    assert!(!path.exists());
    assert_eq!(UpdateState::load(&path), UpdateState::default());
}

#[test]
fn corrupt_state_file_loads_as_default() {
    let path = scratch("corrupt");
    std::fs::write(&path, "{").unwrap();
    assert_eq!(UpdateState::load(&path), UpdateState::default());
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn saved_state_round_trips() {
    let path = scratch("state");
    let state = UpdateState {
        checked_at: 42,
        channel: None,
        available: Some(Available {
            version: "1.9.10".to_owned(),
            channel: "stable".to_owned(),
            date: "2026-11-23".to_owned(),
            notes: vec!["一句更新日志".to_owned()],
        }),
    };
    state.save(&path).unwrap();
    assert_eq!(UpdateState::load(&path), state);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn save_creates_the_parent_directory() {
    let path = scratch("nested/deep/state");
    UpdateState::default().save(&path).unwrap();
    assert!(path.parent().unwrap().exists());
    assert!(path.exists());
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn save_leaves_no_temporary_file_behind() {
    let path = scratch("temporary");
    UpdateState::default().save(&path).unwrap();
    assert!(!path.with_extension("json.tmp").exists());
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn available_requires_every_field() {
    let error = serde_json::from_str::<Available>(r#"{"version": "1.9.10"}"#).unwrap_err();
    assert!(error.to_string().contains("channel"), "{error}");
}

#[test]
fn available_round_trips_through_json() {
    let available = Available {
        version: "1.9.10".to_owned(),
        channel: "beta".to_owned(),
        date: "2026-11-23".to_owned(),
        notes: vec!["第一条".to_owned(), "第二条".to_owned()],
    };
    let text = serde_json::to_string(&available).unwrap();
    assert_eq!(serde_json::from_str::<Available>(&text).unwrap(), available);
}

#[test]
fn asset_defaults_to_no_cpu() {
    let asset: Asset = serde_json::from_str(r#"{"platform": "windows"}"#).unwrap();
    assert_eq!(asset.platform, "windows");
    assert_eq!(asset.cpu, "");
}

#[test]
fn release_defaults_optional_fields() {
    let release: Release =
        serde_json::from_str(r#"{"version": "1.9.10", "channel": "stable"}"#).unwrap();
    assert_eq!(release.version, "1.9.10");
    assert_eq!(release.date, "");
    assert!(release.notes.is_empty());
    assert!(release.assets.is_empty());
}

#[test]
fn release_keeps_assets_in_order() {
    let release: Release = serde_json::from_str(
        r#"{"version": "1.9.10", "channel": "stable",
            "assets": [
                {"platform": "windows", "cpu": "x86_64"},
                {"platform": "linux", "cpu": "arm64"}
            ]}"#,
    )
    .unwrap();
    assert_eq!(release.assets.len(), 2);
    assert_eq!(release.assets[1].cpu, "arm64");
}

#[test]
fn current_target_matches_the_build_platform() {
    let target = Target::current();
    assert!(!target.platform.is_empty());
    assert!(!target.cpu.is_empty());
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        assert_eq!(target.platform, "windows");
        assert_eq!(target.cpu, "x86_64");
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        assert_eq!(target.platform, "linux");
        assert_eq!(target.cpu, "x86_64");
    }
}

#[test]
fn download_url_points_at_the_site() {
    assert_eq!(DOWNLOAD_URL, "https://qingjian.app/download");
}
