//! 设置程序启动：配置文件写坏了也要打得开，下线的取值改写成现在的。

use qingjian_platform::{Config, LocalModel};

use super::Settings;

fn temp_config(name: &str, text: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-settings-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, text).unwrap();
    path
}

/// 语法错、类型错都按缺省值打开并记下错误，文件原样不动；每页都建得出来（顶上多一条提示）。
#[test]
fn a_broken_config_still_opens_with_defaults() {
    for (name, text) in [
        ("syntax", "[general\npage_size = 5\n"),
        ("type", "[general]\npage_size = \"五\"\n"),
    ] {
        let path = temp_config(name, text);
        let settings = Settings::load(path.clone());
        assert!(settings.config_error.is_some(), "{name}");
        assert_eq!(settings.config, Config::default(), "{name}");
        let _ = crate::view::index(&settings);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text, "{name}");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}

#[test]
fn retired_zhiwei_is_rewritten_on_start() {
    let path = temp_config("zhiwei", "[model]\nenabled = true\nname = \"zhiwei\"\n");
    let settings = Settings::load(path.clone());
    assert!(settings.config_error.is_none());
    assert_eq!(settings.config.model.name, LocalModel::Tongbian);
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("name = \"tongbian\"")
    );
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
