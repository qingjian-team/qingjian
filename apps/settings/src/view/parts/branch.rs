//! 带从属项的一行（开了它才有意义的细项）：macOS 缩进一级，Windows 收进这一行的展开卡片。
//! 卡片收着时从属项不建（免得 Tab 走到看不见的控件），建搜索索引时一律展开着建；见 [`bamboo_slip::group_branches`]。

use bamboo_slip::taffy::{AlignItems, FlexDirection, LengthPercentageAuto, NodeId, Size, Style};
use bamboo_slip::{Branch, Look, RenderError, Ui, Visual, group_branches};

use crate::message::Message;
use crate::view::Page;

/// `head` 带上 `children` 建出的从属项；`local` 在这一页里分得开各张卡片就行。
pub fn branch(
    ui: &mut Ui<Message>,
    page: &Page,
    local: &'static str,
    head: NodeId,
    children: impl FnOnce(&mut Ui<Message>) -> Result<Vec<NodeId>, RenderError>,
) -> Result<Branch, RenderError> {
    let key = ui.key(("branch", local));
    let fluent = page.theme.look == Look::Fluent;
    let open = !fluent || page.expand_all || page.settings.expanded.contains(&key);
    let children = if open {
        ui.region(key, children)?
    } else {
        Vec::new()
    };
    Ok(Branch {
        head,
        children,
        expander: fluent.then_some((key, open)),
    })
}

/// 一组带从属项的行，见 [`group_branches`]。
pub fn nested(
    ui: &mut Ui<Message>,
    page: &Page,
    branches: Vec<Branch>,
) -> Result<NodeId, RenderError> {
    group_branches(ui, &page.theme, branches, Message::Expand)
}

/// 只有一个勾选的从属项：勾选靠左，行高与设置行一样（macOS 38、Windows 52 点）。
pub fn check_row(ui: &mut Ui<Message>, page: &Page, check: NodeId) -> Result<NodeId, RenderError> {
    let height = if page.theme.look == Look::Fluent {
        52.0
    } else {
        38.0
    };
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            min_size: Size {
                width: LengthPercentageAuto::auto(),
                height: LengthPercentageAuto::length(page.px(height)),
            },
            ..Style::default()
        },
        Visual::Group,
        &[check],
    )
}
