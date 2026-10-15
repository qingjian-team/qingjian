//! 控件：开关、分段选择、按钮、勾选框、步进框、下拉框、输入框、录键按钮、导航项、搜索框、统计图，以及设置页用的分组卡片、行、段落与对话框。
//!
//! 控件都是普通组件：在 [`crate::Ui`] 里建节点、登记交互，样子全由 [`WidgetTheme`] 给（颜色、圆角、字号、动效），
//! 引擎只带一套中性的缺省主题，品牌外观由使用方换一套主题传进来。尺寸写点，建树时乘主题里的倍数。
//!
//! 写法统一：`控件::new(…).可选设置(…).build(ui, theme, 本地名, 消息)`，本地名在当前作用域下派生稳定 key。

mod button;
mod chart;
mod check_mark;
mod checkbox;
mod chevron;
mod colors;
mod dialog;
mod group;
mod info;
mod info_mark;
mod key_recorder;
mod menu;
mod nav_item;
mod paragraph;
mod parts;
mod scroll_area;
mod search_field;
mod segmented;
mod select;
mod setting_row;
mod stepper;
mod switch;
#[cfg(test)]
mod tests;
mod text_field;
mod theme;

pub use button::Button;
pub use chart::{BarChart, Datum, Heatmap};
pub use checkbox::Checkbox;
pub use colors::WidgetColors;
pub use dialog::dialog;
pub use group::{group, section_title};
pub use info::{info, info_with_link};
pub use key_recorder::KeyRecorder;
pub use menu::Menu;
pub use nav_item::NavItem;
pub use paragraph::paragraph;
pub use scroll_area::ScrollArea;
pub use search_field::SearchField;
pub use segmented::Segmented;
pub use select::Select;
pub use setting_row::SettingRow;
pub use stepper::Stepper;
pub use switch::Switch;
pub use text_field::{Edit, TextEdit, TextField};
pub use theme::WidgetTheme;
