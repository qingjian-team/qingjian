//! 用户导入 Rime 拼音词库的跨文件回归测试，使用自造词条，不携带上游数据。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use qingjian_dictionary::{Dictionary, import::import};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "qingjian-rime-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn load(&self, source: &Path) -> Dictionary {
        let imported = import(source, &self.path.join("output")).unwrap();
        Dictionary::from_path(&imported.path).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn merges_rime_ice_tables_and_encodes_unannotated_words() {
    let fixture = Fixture::new();
    let source = fixture.write(
        "rime_ice.dict.yaml",
        "---\nname: rime_ice\nversion: 'test'\nimport_tables:\n  - cn_dicts/chars # 字表\n  - cn_dicts/base\n  - cn_dicts/words\n...\n",
    );
    fixture.write(
        "cn_dicts/chars.dict.yaml",
        "---\nname: chars\n...\n青\tqing\t100\n简\tjian\t100\n输\tshu\t100\n入\tru\t100\n法\tfa\t100\n",
    );
    fixture.write(
        "cn_dicts/base.dict.yaml",
        "---\nname: base\n...\n青简\tqing jian\t500\n",
    );
    fixture.write(
        "cn_dicts/words.dict.yaml",
        "---\nname: words\ncolumns:\n  - text\n  - weight\n...\n输入法\t123\n不存在\t999\n",
    );
    let dictionary = fixture.load(&source);
    let found = dictionary.lookup(&["shu", "ru", "fa"], false);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].text, "输入法");
    assert_eq!(found[0].frequency, 123);
    assert_eq!(dictionary.len(), 7);
    assert_eq!(dictionary.metadata().unwrap().name, "rime_ice");
    assert_eq!(dictionary.metadata().unwrap().version, "test");
    assert!(dictionary.lookup(&["123"], false).is_empty());
}

#[test]
fn custom_columns_preserve_pinyin_and_frequency() {
    let fixture = Fixture::new();
    let source = fixture.write(
        "columns.dict.yaml",
        "\u{feff}---\nname: '自定义词库'\nlicense: 'CC0-1.0'\ncolumns: [weight, text, code] # 调整列序\n...\n99\t策略\tce lue\n",
    );
    let dictionary = fixture.load(&source);
    let found = dictionary.lookup(&["ce", "lve"], false);
    assert_eq!(found[0].text, "策略");
    assert_eq!(found[0].frequency, 99);
    assert_eq!(dictionary.metadata().unwrap().name, "自定义词库");
    assert_eq!(dictionary.metadata().unwrap().license, "CC0-1.0");
}

#[test]
fn first_table_wins_and_cycles_are_read_once() {
    let fixture = Fixture::new();
    let source = fixture.write(
        "main.dict.yaml",
        "---\nname: main\nimport_tables: [first, nested/second]\n...\n",
    );
    fixture.write(
        "first.dict.yaml",
        "---\nname: first\n...\n策略\tce lue\t10\n",
    );
    fixture.write(
        "nested/second.dict.yaml",
        "---\nname: second\nimport_tables: [main, first]\n...\n策略\tce lve\t999\n",
    );
    let dictionary = fixture.load(&source);
    let found = dictionary.lookup(&["ce", "lve"], false);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].frequency, 10);
}

#[test]
fn rare_readings_are_excluded_only_from_automatic_encoding() {
    let fixture = Fixture::new();
    let source = fixture.write(
        "readings.dict.yaml",
        "---\nname: readings\n...\n的\tde\t96\n的\tdi\t4\n确\tque\t100\n的确\n的确\tdi que\t10\n",
    );
    let dictionary = fixture.load(&source);
    assert_eq!(dictionary.lookup(&["de", "que"], false)[0].text, "的确");
    let explicit = dictionary.lookup(&["di", "que"], false);
    assert_eq!(explicit.len(), 1);
    assert_eq!(explicit[0].frequency, 10);
    assert_eq!(dictionary.lookup(&["di"], false)[0].text, "的");
}

#[test]
fn missing_table_does_not_replace_an_existing_import() {
    let fixture = Fixture::new();
    let source = fixture.write("main.dict.yaml", "---\nname: main\n...\n青简\tqing jian\n");
    let dest = fixture.path.join("output");
    let imported = import(&source, &dest).unwrap();
    let original = std::fs::read(&imported.path).unwrap();
    fixture.write(
        "main.dict.yaml",
        "---\nname: main\nimport_tables: [missing]\n...\n青简\tqing jian\n",
    );
    assert!(import(&source, &dest).is_err());
    assert_eq!(std::fs::read(&imported.path).unwrap(), original);
}

#[test]
fn rejects_imports_outside_the_selected_dictionary_directory() {
    let fixture = Fixture::new();
    fixture.write(
        "outside.dict.yaml",
        "---\nname: outside\n...\n青简\tqing jian\n",
    );
    let source = fixture.write(
        "inside/main.dict.yaml",
        "---\nname: main\nimport_tables: [../outside]\n...\n",
    );
    assert!(import(&source, &fixture.path.join("output")).is_err());
}
