use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuItem};
use objc2_foundation::NSString;
use qingjian_core::FuzzyRules;
use qingjian_platform::Config;

use super::MenuAction;
use super::target::MenuTarget;

/// 菜单本体与需要按状态刷新的那几项。
pub struct InputMenu {
    /// 菜单。挂到状态项和 IMK `menu` 回调的是同一个对象。
    menu: Retained<NSMenu>,

    /// 「云联想」勾选项。
    cloud: Retained<NSMenuItem>,

    /// 模糊音子菜单的九条勾选项，顺序同 [`FuzzyRules::NAMES`]。
    fuzzy: Vec<Retained<NSMenuItem>>,

    /// 配置文件解析失败时显示的提示行，平时隐藏。
    error: Retained<NSMenuItem>,

    /// 所有条目的 target，要和菜单活得一样久。
    _target: Retained<MenuTarget>,
}

impl InputMenu {
    pub fn new(mtm: MainThreadMarker, version: &str) -> Self {
        let target = MenuTarget::new(mtm);
        let menu = NSMenu::new(mtm);
        // 不让 AppKit 按响应链判断可用性：它找不到 target 就会把整份菜单灰掉
        menu.setAutoenablesItems(false);

        let cloud = action_item(mtm, "云联想", Some(MenuAction::ToggleCloud), &target);
        menu.addItem(&cloud);

        let fuzzy_menu = NSMenu::new(mtm);
        fuzzy_menu.setAutoenablesItems(false);
        let fuzzy: Vec<_> = FuzzyRules::NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let item = action_item(
                    mtm,
                    &fuzzy_label(name),
                    Some(MenuAction::ToggleFuzzy(index)),
                    &target,
                );
                fuzzy_menu.addItem(&item);
                item
            })
            .collect();
        let fuzzy_parent = action_item(mtm, "模糊音", None, &target);
        fuzzy_parent.setSubmenu(Some(&fuzzy_menu));
        menu.addItem(&fuzzy_parent);

        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&action_item(
            mtm,
            "偏好设置…",
            Some(MenuAction::OpenPreferences),
            &target,
        ));
        menu.addItem(&action_item(
            mtm,
            "打开日志目录",
            Some(MenuAction::OpenLogs),
            &target,
        ));
        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let error = action_item(mtm, "", None, &target);
        error.setEnabled(false);
        error.setHidden(true);
        menu.addItem(&error);
        let about = action_item(mtm, &format!("青简 {version}"), None, &target);
        about.setEnabled(false);
        menu.addItem(&about);

        Self {
            menu,
            cloud,
            fuzzy,
            error,
            _target: target,
        }
    }

    /// 给状态项 / IMK 回调用的菜单对象。
    pub fn ns_menu(&self) -> Retained<NSMenu> {
        self.menu.clone()
    }

    /// 按当前配置刷新勾选状态。`cloud_active` 是 Engine 里真接上了 Predictor：
    /// 配置开了但没接上（多半是没密钥）时不打勾，标题说明原因，不能显示开了实际没开。
    pub fn sync(&self, config: &Config, cloud_active: bool, error: Option<&str>) {
        let title = match (config.predict.enabled, cloud_active) {
            (true, false) => "云联想（启用失败，见日志）",
            _ => "云联想",
        };
        self.cloud.setTitle(&NSString::from_str(title));
        set_checked(&self.cloud, cloud_active);
        for (item, name) in self.fuzzy.iter().zip(FuzzyRules::NAMES) {
            set_checked(item, config.fuzzy.is_on(name));
        }
        match error {
            Some(message) => {
                self.error
                    .setTitle(&NSString::from_str(&format!("配置文件有错误：{message}")));
                self.error.setHidden(false);
            }
            None => self.error.setHidden(true),
        }
    }
}

/// 建一个菜单项。`action` 为 `None` 的是纯展示项（子菜单父项、关于行）。
fn action_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<MenuAction>,
    target: &MenuTarget,
) -> Retained<NSMenuItem> {
    // SAFETY: 选择器与 MenuTarget / 控制器上定义的 `menuAction:` 一致，签名 (id) -> void
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(title),
            action.map(|_| sel!(menuAction:)),
            &NSString::from_str(""),
        )
    };
    if let Some(action) = action {
        unsafe { item.setTarget(Some(target)) };
        item.setTag(action.tag());
    }
    item
}

fn set_checked(item: &NSMenuItem, on: bool) {
    item.setState(if on {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
}

/// `an_ang` → `an = ang`。
fn fuzzy_label(name: &str) -> String {
    name.replace('_', " = ")
}
