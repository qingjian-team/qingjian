//! 原生雾凇集成测试的隔离目录，不依赖用户安装或网络。

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    pub root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qingjian-native-ice-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        for directory in ["cn_dicts", "en_dicts", "opencc", "lua"] {
            std::fs::create_dir(root.join(directory)).unwrap();
        }
        let fixture = Self { root };
        fixture.write(
            "rime_ice.dict.yaml",
            "---\nname: rime_ice\nimport_tables: [cn_dicts/8105, cn_dicts/words]\n...\n",
        );
        fixture.write("cn_dicts/8105.dict.yaml","---\nname: chars\n...\n你\tni\t100\n好\thao\t100\n休\txiu\t100\n开\tkai\t100\n发\tfa\t100\n给\tgei\t100\n予\tyu\t100\n");
        fixture.write(
            "cn_dicts/words.dict.yaml",
            "---\nname: words\n...\n你好\tni hao\t1000\n开发\tkai fa\t1000\n给予\tgei yu\t1000\n",
        );
        fixture.write("rime_ice.schema.yaml","date_translator: {date: rq, time: sj, week: xq, datetime: dt, timestamp: ts, datezh: rqzh, dateen: rqen}\nlunar: nl\nuuid: uuid\npin_cand_filter: [\"d\\t的\"]\nlong_word_filter: {count: 2, idx: 4}\nreduce_english_filter: {words: [rug], idx: 2}\n");
        fixture.write("symbols_v.yaml","symbols:\n  vhelp: [符号列表]\n  vsx: [±, ÷, ×]\n  va: [ā, á, ǎ, à]\n  v1: [一, 壹, ①]\n");
        fixture.write("custom_phrase.txt", "青简\tqj\t1\n");
        fixture.write(
            "en_dicts/en.dict.yaml",
            "---\nname: en\n...\nhello\thello\t100\nrug\trug\t100\n",
        );
        fixture.write(
            "en_dicts/en_ext.dict.yaml",
            "---\nname: ext\n...\nRust\trust\t100\n",
        );
        fixture.write("en_dicts/cn_en.txt", "3D打印\t3ddayin\nX光\txguang\n");
        fixture.write("opencc/emoji.txt", "你好\t你好 👋\n");
        fixture.write(
            "radical_pinyin.dict.yaml",
            "---\nname: radical\n...\n㑄\tren'mu\t1\n休\tren'mu\t100\n",
        );
        fixture.write(
            "lua/corrector.lua",
            "[\"gei yu\"] = { text = \"给予\", comment = \"jǐ yǔ\" },\n",
        );
        fixture
    }

    pub fn write(&self, path: &str, text: &str) {
        std::fs::write(self.root.join(path), text).unwrap();
    }

    pub fn engine(&self) -> Engine {
        let mut engine = Engine::new(Dictionary::parse("原引擎\tyuan yin qing\t100\n").unwrap());
        engine
            .load_rime_ice(&self.root, &self.root.join("cache"))
            .unwrap();
        engine
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
