//! 改 `config.toml` / 用户 `dicts/` 后，下一次 tick 就进入正在运行的 Router。

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use qingjian_core::Engine;
use qingjian_dictionary::{Dictionary, import};
use qingjian_platform::Config;

use super::CONFIG_POLL_INTERVAL;
use crate::dispatch::{Router, RouterConfig};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-reload-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 按 `config.toml` 起一个开了热加载的 Router。
fn watched(dir: &Path, source: &str) -> Router {
    let config_path = dir.join("config.toml");
    write_config(&config_path, source, 100);
    let config = Config::load(&config_path).unwrap();
    let mut router = Router::new(
        Engine::new(Dictionary::default()),
        RouterConfig::from(&config),
    );
    router.watch_config(&config, config_path, dir.to_owned(), dir.to_owned());
    router
}

/// 写文件并钉死 mtime：同一秒内连写两次时文件系统的 mtime 可能不变。
fn write_config(path: &Path, source: &str, seconds: u64) {
    std::fs::write(path, source).unwrap();
    set_mtime(path, seconds);
}

fn set_mtime(path: &Path, seconds: u64) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
        .unwrap();
}

/// 跳过一秒节流，直接看一次。
fn poll(router: &mut Router) {
    router.reload.as_mut().unwrap().last_check = Instant::now() - CONFIG_POLL_INTERVAL;
    router.tick();
}

#[test]
fn edited_config_applies_on_next_tick() {
    let dir = temp_dir("apply");
    let mut router = watched(&dir, "[general]\npage_size = 5\n");
    assert_eq!(router.config.page_size, 5);
    assert_eq!(router.engine.shuangpin(), None);

    write_config(
        &dir.join("config.toml"),
        "[general]\npage_size = 7\nscheme = \"xiaohe\"\nchinese_first = false\n",
        200,
    );
    poll(&mut router);
    assert_eq!(router.config.page_size, 7);
    assert!(router.engine.shuangpin().is_some());
    assert!(!router.engine.chinese_first());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn broken_config_keeps_previous_settings() {
    let dir = temp_dir("broken");
    let mut router = watched(&dir, "[general]\npage_size = 6\n");

    write_config(&dir.join("config.toml"), "[general\npage_size = 3\n", 200);
    poll(&mut router);
    assert_eq!(router.config.page_size, 6);

    // 修好后照常生效
    write_config(&dir.join("config.toml"), "[general]\npage_size = 3\n", 300);
    poll(&mut router);
    assert_eq!(router.config.page_size, 3);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn user_dictionary_import_applies_without_config_change() {
    let dir = temp_dir("dicts");
    let mut router = watched(&dir, "");
    assert!(router.engine.extra_dictionaries().is_empty());

    let source = dir.join("law.dict.yaml");
    std::fs::write(&source, "---\nname: law\n...\n合同法\the tong fa\t120\n").unwrap();
    let imported = import::import(&source, &dir.join("dicts")).unwrap();
    set_mtime(&imported.path, 100);
    poll(&mut router);
    let dictionaries = router.engine.extra_dictionaries();
    assert_eq!(dictionaries.len(), 1);
    assert_eq!(
        dictionaries[0].lookup(&["he", "tong", "fa"], false)[0].text,
        "合同法"
    );

    std::fs::remove_file(&imported.path).unwrap();
    poll(&mut router);
    assert!(router.engine.extra_dictionaries().is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}
