//! 启动与监听建立之间恢复配置、临时删除和重建都不能永久关闭原功能。

use std::time::Instant;

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::{Config, NumpadDigit};

use crate::dispatch::{DataDirs, Router, RouterConfig};

fn poll(router: &mut Router) {
    router.reload.as_mut().unwrap().last_check = Instant::now() - super::CONFIG_POLL_INTERVAL;
    router.poll_config_reload();
}

#[test]
fn recovered_startup_and_temporary_missing_config_preserve_and_reload_settings() {
    let dir = std::env::temp_dir().join(format!("qingjian-config-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    let old = Config::default();
    let mut router = Router::new(Engine::new(Dictionary::default()), RouterConfig::from(&old));
    // 模拟启动读取失败后，文件在 watcher 建立前已恢复。
    std::fs::write(
        &path,
        "[general]\nnumpad_digit = 'direct'\n[offline_translation]\nenabled = true\n",
    )
    .unwrap();
    router.watch_config(&old, path.clone(), dir.clone(), DataDirs::default());
    poll(&mut router);
    assert_eq!(router.config.numpad_digit, NumpadDigit::Direct);
    assert!(router.reload.as_ref().unwrap().applied_offline_translation);
    std::fs::remove_file(&path).unwrap();
    poll(&mut router);
    assert_eq!(router.config.numpad_digit, NumpadDigit::Direct);
    assert!(router.reload.as_ref().unwrap().applied_offline_translation);
    std::fs::write(&path, "[general]\nnumpad_digit = 'select'\n").unwrap();
    poll(&mut router);
    assert_eq!(router.config.numpad_digit, NumpadDigit::Select);
    assert!(!router.reload.as_ref().unwrap().applied_offline_translation);
    drop(router);
    std::fs::remove_dir_all(&dir).unwrap();
}
