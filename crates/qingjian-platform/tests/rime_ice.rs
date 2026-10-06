//! 原生全拼配置的加载、失败回退与关闭。

#[path = "../../qingjian-core/tests/ice_support/mod.rs"]
mod ice_support;

use ice_support::Fixture;
use qingjian_platform::{Config, RimeIceConfig};

#[test]
fn profile_config_defaults_and_atomic_load() {
    let fixture = Fixture::new();
    let path = fixture.path().join("config.toml");
    Config::write_template_if_missing(&path).unwrap();
    let config = Config::load(&path).unwrap();
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("[rime_ice]")
    );
    assert_eq!(config.rime_ice, RimeIceConfig::default());
    let mut engine = fixture.engine();
    engine.disable_rime_ice();
    let cache = fixture.path().join("cache");
    let valid = RimeIceConfig {
        enabled: true,
        data_dir: fixture.path().to_owned(),
    };
    valid.apply(&mut engine, &cache).unwrap();
    assert!(engine.rime_ice_active());
    let invalid = RimeIceConfig {
        enabled: true,
        data_dir: "relative/path".into(),
    };
    assert!(invalid.apply(&mut engine, &cache).is_err());
    assert!(engine.rime_ice_active());
    RimeIceConfig::default().apply(&mut engine, &cache).unwrap();
    assert!(!engine.rime_ice_active());
    engine.set_input("yuanyinqing");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|c| c.text == "原引擎")
    );
}
