//! 「模糊音」页：各组勾选，三列排布。配置键与 `FuzzyRules` 字段名一致。
//! 用 CheckBox 不用 ToggleSwitch：Reactor 0.100 的 ToggleSwitch 去不掉「开/关」字样，多列会挤。

use windows_reactor::*;

use crate::panel::controls::{note, page};
use crate::panel::{Message, Settings};

fn cell(label: &str, key: &'static str, on: bool, context: &mut ViewContext<Settings>) -> View {
    CheckBox::new()
        .is_checked(on)
        .on_is_checked_changed(context.callback(move |value| Message::Fuzzy(key, value)))
        .width(150.0)
        .content(label)
}

fn row(cells: [View; 3]) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children(cells)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let f = &settings.config.fuzzy;
    let body = StackPanel::new().spacing(8.0).children([
        row([
            cell("z ↔ zh", "z_zh", f.z_zh, context),
            cell("c ↔ ch", "c_ch", f.c_ch, context),
            cell("s ↔ sh", "s_sh", f.s_sh, context),
        ]),
        row([
            cell("n ↔ l", "n_l", f.n_l, context),
            cell("f ↔ h", "f_h", f.f_h, context),
            cell("l ↔ r", "l_r", f.l_r, context),
        ]),
        row([
            cell("an ↔ ang", "an_ang", f.an_ang, context),
            cell("en ↔ eng", "en_eng", f.en_eng, context),
            cell("in ↔ ing", "in_ing", f.in_ing, context),
        ]),
        note(
            "勾上的两种读音互相通用（比如开 z ↔ zh，敲 zi 也出 zhi 的字），模糊命中的词排在准确命中之后。\
             an ↔ ang 含 ian/iang、uan/uang。缺省全关。",
        ),
    ]);
    page("模糊音", body)
}
