//! 简繁状态落盘、设置热加载和旧协议兼容。
use super::CONFIG_POLL_INTERVAL;
use crate::dispatch::{DataDirs, Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::Config;
use qingjian_platform::protocol::{KeyEvent, KeyModifiers};
use std::time::Instant;

#[test]
fn traditional_state_persists_and_shortcut_reload_reaches_dll() {
    let dir = std::env::temp_dir().join(format!("qingjian-traditional-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "# preserved comment\n[general]\ntraditional = false\n[shortcut]\ntoggle_traditional = \"ctrl+shift+f\"\n").unwrap();
    let config = Config::load(&path).unwrap();
    let mut router = Router::new(
        Engine::new(Dictionary::default()),
        RouterConfig::from(&config),
    );
    router.watch_config(&config, path.clone(), dir.clone(), DataDirs::default());
    router.apply_key(&KeyEvent::new(
        0x46,
        None,
        KeyModifiers {
            ctrl: true,
            shift: true,
            ..Default::default()
        },
    ));
    assert!(Config::load(&path).unwrap().general.traditional);
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("# preserved comment")
    );
    Config::set_value(&path, "shortcut", "toggle_traditional", "ctrl+alt+g").unwrap();
    router.reload.as_mut().unwrap().last_mtime = None;
    router.reload.as_mut().unwrap().last_check = Instant::now() - CONFIG_POLL_INTERVAL;
    router.poll_config_reload();
    assert_eq!(
        router.input_settings().toggle_traditional,
        Some("ctrl+alt+g".parse().unwrap())
    );
    assert!(router.config.traditional);
    Config::set_value(&path, "shortcut", "toggle_traditional", "").unwrap();
    router.reload.as_mut().unwrap().last_mtime = None;
    router.reload.as_mut().unwrap().last_check = Instant::now() - CONFIG_POLL_INTERVAL;
    router.poll_config_reload();
    assert!(router.input_settings().toggle_traditional.is_none());
    drop(router);
    std::fs::remove_dir_all(dir).unwrap();
}
