//! 测试：设置程序能写的每个键，输入法的 `Config` 都读得到、值对得上。
//!
//! 离屏建每一页（窗口开得够高，不用滚），逐个点控件、点开每个下拉菜单逐项点，只放行改配置的消息交给 `update`
//! （打开文件、导出日志、清空输入日志、移除词库、联网、短语这些碰真数据的一律不放）；每点一下比对配置文件前后，
//! 变了的键用 `Config` 读回来再序列化，键名不认或值被改写就算错。配置是临时目录里的一份，不碰真配置。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bamboo_slip::{
    App, Edit, Event, FontLibrary, Interaction, KeyCode, KeyPress, Modifiers, TextInput,
    TextPainter, Ui,
};
use qingjian_platform::{Config, KeyCombo};

use crate::message::{FieldId, Message, Popup};
use crate::page_id::PageId;
use crate::settings::Settings;
use crate::shortcut::Shortcut;
use crate::view;

const SIZE: (u32, u32) = (1640, 8000);

struct Harness {
    settings: Settings,

    interaction: Interaction<Message>,

    text: TextPainter,

    path: PathBuf,

    /// 不对的键与原因。
    problems: BTreeMap<String, String>,

    /// 写过、核对过的键。
    checked: BTreeMap<String, ()>,
}

impl Harness {
    fn frame(&mut self) {
        let mut ui = Ui::new(&self.interaction);
        let root = view::view(&mut ui, &self.settings, 2.0, SIZE).unwrap();
        ui.attach_overlays(root).unwrap();
        let (mut scene, elements) = ui.finish();
        scene.layout(root, &mut self.text).unwrap();
        self.interaction.attach(&scene, root, elements).unwrap();
    }

    /// 点一下，返回产出的消息（不处理）。
    fn click(&mut self, key: bamboo_slip::Key) -> Vec<Message> {
        let Some((x, y, width, height)) = self.interaction.rect(key) else {
            return Vec::new();
        };
        self.interaction.handle(&Event::PointerMoved {
            x: x + width / 2.0,
            y: y + height / 2.0,
        });
        self.interaction.handle(&Event::PointerDown);
        self.interaction.handle(&Event::PointerUp).messages
    }

    /// 只放行改配置的消息；每条都比对配置前后、核对变了的键。
    fn apply(&mut self, messages: Vec<Message>) {
        for message in messages {
            let writes = matches!(
                message,
                Message::Set(..)
                    | Message::SetList(..)
                    | Message::Size(..)
                    | Message::ResetSizes
                    | Message::ModeKey(..)
                    | Message::Recorded(..)
                    | Message::ResetShortcuts
                    | Message::Edit(..)
                    | Message::Commit(..)
                    | Message::OpenMenu(..)
                    | Message::CloseMenu
            );
            if !writes {
                continue;
            }
            let before = read(&self.path);
            self.settings.update(message);
            let after = read(&self.path);
            self.verify(&before, &after);
        }
    }

    fn verify(&mut self, before: &toml::Table, after: &toml::Table) {
        let loaded = Config::load(&self.path).map(|config| toml::Table::try_from(&config));
        let loaded = match loaded {
            Ok(Ok(table)) => table,
            Ok(Err(error)) => panic!("Config 序列化失败：{error}"),
            Err(error) => {
                self.problems
                    .insert("（整份）".to_owned(), format!("输入法读不了：{error}"));
                return;
            }
        };
        for (section, keys) in after {
            let Some(keys) = keys.as_table() else {
                continue;
            };
            for (key, value) in keys {
                let old = before.get(section).and_then(|table| table.get(key));
                if old == Some(value) {
                    continue;
                }
                let name = format!("[{section}] {key}");
                let read_back = loaded.get(section).and_then(|table| table.get(key));
                match read_back {
                    None => {
                        self.problems
                            .insert(name, "写进了配置，输入法的 Config 里没有这个键".to_owned());
                    }
                    Some(read_back) if !same(read_back, value) => {
                        self.problems
                            .insert(name, format!("写的是 {value}，输入法读成 {read_back}"));
                    }
                    Some(_) => {
                        self.checked.insert(name, ());
                    }
                }
            }
        }
    }

    fn sweep_page(&mut self, page: PageId) {
        self.settings.page = page;
        self.settings.menu = None;
        self.frame();
        let keys: Vec<_> = self.interaction.keys().collect();
        for key in &keys {
            self.frame();
            let messages = self.click(*key);
            self.apply(messages);
            if let Some(popup) = self.settings.menu {
                self.sweep_menu(popup, &keys);
            }
        }
    }

    /// 菜单开着时多出来的元素就是菜单项：每项重新打开菜单再点。
    fn sweep_menu(&mut self, popup: Popup, page_keys: &[bamboo_slip::Key]) {
        self.frame();
        let items: Vec<_> = self
            .interaction
            .keys()
            .filter(|key| !page_keys.contains(key))
            .collect();
        for item in items {
            self.settings.menu = Some(popup);
            self.frame();
            let messages = self.click(item);
            self.apply(messages);
        }
        self.settings.menu = None;
    }
}

fn read(path: &Path) -> toml::Table {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or_default()
}

/// 整数与同值的浮点数算一样（`Config` 里是 f32 的键，文件里可能写成整数）；修饰键与组合键按解析结果比，
/// Windows 的写法（ctrl / alt / win）读回来是等价的 control / option / command。
fn same(a: &toml::Value, b: &toml::Value) -> bool {
    match (a, b) {
        (toml::Value::String(x), toml::Value::String(y)) if x != y => {
            matches!((x.parse::<qingjian_platform::Modifiers>(), y.parse::<qingjian_platform::Modifiers>()), (Ok(p), Ok(q)) if p == q)
                || matches!((x.parse::<KeyCombo>(), y.parse::<KeyCombo>()), (Ok(p), Ok(q)) if p == q)
        }
        (toml::Value::Integer(i), toml::Value::Float(f))
        | (toml::Value::Float(f), toml::Value::Integer(i)) => (*i as f64 - f).abs() < 1e-6,
        (toml::Value::Float(x), toml::Value::Float(y)) => (x - y).abs() < 1e-4,
        _ => a == b,
    }
}

#[test]
fn every_key_the_settings_app_writes_is_read_by_the_ime() {
    let dir = std::env::temp_dir().join(format!("qingjian-settings-keys-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    let _ = std::fs::remove_file(&path);
    let mut settings = Settings::load(path.clone()).unwrap();
    let mut text = TextPainter::new(FontLibrary::system("zh-CN").unwrap());
    settings.setup(&mut text);
    let mut harness = Harness {
        settings,
        interaction: Interaction::new(),
        text,
        path,
        problems: BTreeMap::new(),
        checked: BTreeMap::new(),
    };
    // 点两轮：第一轮打开的开关会让依赖它的控件可用，第二轮把它们也点到
    for _ in 0..2 {
        for page in PageId::ALL {
            harness.sweep_page(page);
        }
    }
    // 输入框与录键不是点出来的：直接发编辑与按键
    for (id, text) in [
        (FieldId::BaseUrl, "https://example.com/v1"),
        (FieldId::Model, "test-model"),
        (FieldId::ApiKey, "sk-test"),
        (FieldId::LiveBaseUrl, "http://127.0.0.1:11434"),
        (FieldId::LiveModel, "test-local-model"),
    ] {
        harness.apply(vec![
            Message::Edit(id, Edit::Text(TextInput::Insert(text.to_owned()))),
            Message::Commit(id),
        ]);
    }
    let presses = [
        KeyPress::new(
            KeyCode::Character("j".to_owned()),
            Modifiers {
                command: true,
                shift: true,
                ..Modifiers::default()
            },
        ),
        KeyPress::new(KeyCode::Character(";".to_owned()), Modifiers::default()),
    ];
    for shortcut in Shortcut::ALL {
        for press in &presses {
            harness.apply(vec![Message::Recorded(shortcut, press.clone())]);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        harness.problems.is_empty(),
        "这些键输入法读不到或读错：{:#?}",
        harness.problems
    );
    // 至少要真点到过各页的主要控件，免得点不中时测试空过
    assert!(
        harness.checked.len() >= 40,
        "只核对到 {} 个键：{:#?}",
        harness.checked.len(),
        harness.checked.keys()
    );
}
