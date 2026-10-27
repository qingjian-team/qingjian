//! 设置页的视图函数：竹青侧栏（搜索、线性图标的导航、竹节标记）+ 分组卡片里的「候选窗口」设置，控件全用引擎的控件库。

use std::sync::Arc;

use bamboo_slip::taffy::{
    Dimension, FlexDirection, JustifyContent, LengthPercentage, LengthPercentageAuto, NodeId,
    Position, Rect, Size, Style,
};
use bamboo_slip::{
    Button, Checkbox, Interact, KeyCode, NavItem, RenderError, ScrollArea, SearchField, Segmented,
    Select, SettingRow, Stepper, Switch, TITLEBAR, Ui, Visual, WidgetTheme, group, section_title,
};

use crate::demo::{
    APPEARANCES, Choice, Demo, ENGINES, LAYOUTS, Message, PAGES, PINYIN, Size as Font,
};
use crate::icons::{Glyph, PAGE_GLYPHS};
use crate::palette::{DARK, LIGHT, Palette};

/// 建整窗的场景树，根节点就是窗口大小（像素）。
pub fn view(
    ui: &mut Ui<Message>,
    demo: &Demo,
    scale: f32,
    (width, height): (u32, u32),
) -> Result<NodeId, RenderError> {
    let palette = if demo.dark() { &DARK } else { &LIGHT };
    let page = Page {
        theme: WidgetTheme::new(palette.widgets, scale),
        palette,
        demo,
    };
    let sidebar = page.sidebar(ui)?;
    let content = page.content(ui)?;
    let root = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(width as f32),
                height: Dimension::length(height as f32),
            },
            flex_direction: FlexDirection::Row,
            ..Style::default()
        },
        Visual::solid(palette.window, 0.0),
        &[sidebar, content],
    )?;
    let key = ui.key("root");
    ui.interact(
        root,
        key,
        Interact::new().on_key(|press| (press.key == KeyCode::Escape).then_some(Message::Quit)),
    );
    Ok(root)
}

struct Page<'a> {
    theme: WidgetTheme,

    palette: &'a Palette,

    demo: &'a Demo,
}

impl Page<'_> {
    fn px(&self, points: f32) -> f32 {
        self.theme.px(points)
    }

    fn sidebar(&self, ui: &mut Ui<Message>) -> Result<NodeId, RenderError> {
        let theme = &self.theme;
        let search = SearchField::new(&self.demo.search)
            .placeholder("搜索")
            .icon(Arc::new(Glyph::Search))
            .build(ui, theme, "search", Message::Search, None, None)?;
        let items = ui.scope("nav", |ui| -> Result<Vec<NodeId>, RenderError> {
            let mut items = Vec::new();
            for (index, name) in PAGES.iter().enumerate() {
                let selected = index == self.demo.page;
                let mut item = NavItem::new(name)
                    .icon(Arc::new(PAGE_GLYPHS[index]))
                    .selected(selected);
                if selected {
                    item = item.accessory(self.joint(ui)?);
                }
                items.push(item.build(ui, theme, index, Message::Page(index))?);
            }
            Ok(items)
        })?;
        let list = ui.scene().node(
            Style {
                flex_direction: FlexDirection::Column,
                gap: self.gap(1.0),
                margin: Rect {
                    top: LengthPercentageAuto::length(self.px(12.0)),
                    ..Rect::zero()
                },
                ..Style::default()
            },
            Visual::Group,
            &items,
        )?;
        ui.scene().node(
            Style {
                size: Size {
                    width: Dimension::length(self.px(216.0)),
                    height: Dimension::auto(),
                },
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                padding: Rect {
                    top: LengthPercentage::length(self.px(TITLEBAR + 14.0)),
                    ..self.padding(16.0, 14.0)
                },
                ..Style::default()
            },
            Visual::solid(self.palette.sidebar, 0.0),
            &[search, list],
        )
    }

    /// 当前页的竹节：选中项左边一段中间断开的竖线（品牌元素，由使用方叠到导航项上）。
    fn joint(&self, ui: &mut Ui<Message>) -> Result<NodeId, RenderError> {
        let (thick, length) = (self.px(2.0), self.px(16.0));
        let part = length * 0.42;
        let mut pieces = Vec::new();
        for top in [0.0, length - part] {
            pieces.push(ui.scene().node(
                Style {
                    position: Position::Absolute,
                    size: Size {
                        width: Dimension::length(thick),
                        height: Dimension::length(part),
                    },
                    inset: Rect {
                        left: LengthPercentageAuto::length(0.0),
                        top: LengthPercentageAuto::length(top),
                        right: LengthPercentageAuto::auto(),
                        bottom: LengthPercentageAuto::auto(),
                    },
                    ..Style::default()
                },
                Visual::solid(self.theme.colors.accent, thick / 2.0),
                &[],
            )?);
        }
        let column = ui.scene().node(
            Style {
                size: Size {
                    width: Dimension::length(thick),
                    height: Dimension::length(length),
                },
                ..Style::default()
            },
            Visual::Group,
            &pieces,
        )?;
        ui.scene().node(
            Style {
                position: Position::Absolute,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::CENTER,
                inset: Rect {
                    left: LengthPercentageAuto::length(-self.px(9.0)),
                    top: LengthPercentageAuto::length(0.0),
                    right: LengthPercentageAuto::auto(),
                    bottom: LengthPercentageAuto::length(0.0),
                },
                ..Style::default()
            },
            Visual::Group,
            &[column],
        )
    }

    fn content(&self, ui: &mut Ui<Message>) -> Result<NodeId, RenderError> {
        let mut children = Vec::new();
        if self.demo.page == 1 {
            children.extend(self.candidates(ui)?);
        }
        let column = ui.scene().node(
            Style {
                flex_direction: FlexDirection::Column,
                gap: self.gap(10.0),
                padding: Rect {
                    top: LengthPercentage::length(self.px(TITLEBAR + 12.0)),
                    ..self.padding(28.0, 36.0)
                },
                ..Style::default()
            },
            Visual::Group,
            &children,
        )?;
        // 每页各记各的滚动位置
        ScrollArea::new(Style {
            flex_grow: 1.0,
            ..Style::default()
        })
        .build(ui, &self.theme, ("content", self.demo.page), column)
    }

    /// 「候选窗口」页：外观、排布、文字、动效四组。
    fn candidates(&self, ui: &mut Ui<Message>) -> Result<Vec<NodeId>, RenderError> {
        let theme = &self.theme;
        let demo = self.demo;
        let mut out = Vec::new();

        let appearance = self.choice(ui, Choice::Appearance, &APPEARANCES)?;
        let row = SettingRow::new("外观")
            .control(appearance)
            .build(ui, theme)?;
        out.push(section_title(ui, theme, "外观")?);
        out.push(group(ui, theme, &[row])?);

        let layout = self.choice(ui, Choice::Layout, &LAYOUTS)?;
        let expand = Checkbox::new(demo.expand, "横排时 ↑ / ↓ 展开成多行")
            .disabled(demo.layout == 0)
            .build(ui, theme, "expand", Message::Expand)?;
        let layout = SettingRow::new("排布")
            .control(layout)
            .below(expand)
            .build(ui, theme)?;
        let pinyin = self.choice(ui, Choice::Pinyin, &PINYIN)?;
        let pinyin = SettingRow::new("拼音显示位置")
            .control(pinyin)
            .build(ui, theme)?;
        out.push(section_title(ui, theme, "排布")?);
        out.push(group(ui, theme, &[layout, pinyin])?);

        let engine = self.choice(ui, Choice::Engine, &ENGINES)?;
        let engine = SettingRow::new("渲染引擎")
            .control(engine)
            .build(ui, theme)?;
        let custom = demo.engine == 0;
        let font = Select::new("系统字体")
            .width(180.0)
            .disabled(!custom)
            .build(ui, theme, "font", Message::OpenFonts)?;
        let font = SettingRow::new("字体").control(font).build(ui, theme)?;
        let candidate = self.size(ui, Font::Candidate, !custom)?;
        let candidate = SettingRow::new("候选字号")
            .control(candidate)
            .build(ui, theme)?;
        let gloss = self.size(ui, Font::Gloss, !custom)?;
        let reset = Button::new("恢复主题字号").disabled(!custom).build(
            ui,
            theme,
            "reset-sizes",
            Message::ResetSizes,
        )?;
        let gloss = SettingRow::new("译文字号")
            .control(gloss)
            .below(reset)
            .build(ui, theme)?;
        out.push(section_title(ui, theme, "文字")?);
        out.push(group(ui, theme, &[engine, font, candidate, gloss])?);

        let animations =
            Switch::new(demo.animations).build(ui, theme, "animations", Message::Animations)?;
        let animations = SettingRow::new("过渡动画")
            .control(animations)
            .build(ui, theme)?;
        out.push(section_title(ui, theme, "动效")?);
        out.push(group(ui, theme, &[animations])?);
        Ok(out)
    }

    fn choice(
        &self,
        ui: &mut Ui<Message>,
        choice: Choice,
        options: &[&str],
    ) -> Result<NodeId, RenderError> {
        Segmented::new(options, self.demo.choice(choice)).build(
            ui,
            &self.theme,
            format!("{choice:?}"),
            move |index| Message::Choose(choice, index),
        )
    }

    fn size(
        &self,
        ui: &mut Ui<Message>,
        size: Font,
        disabled: bool,
    ) -> Result<NodeId, RenderError> {
        let (down, up) = self.demo.size_limits(size);
        Stepper::new(&format!("{} pt", self.demo.size(size)))
            .limits(down, up)
            .disabled(disabled)
            .build(
                ui,
                &self.theme,
                format!("{size:?}"),
                Message::Step(size, -1),
                Message::Step(size, 1),
            )
    }

    fn padding(&self, vertical: f32, horizontal: f32) -> Rect<LengthPercentage> {
        let (v, h) = (
            LengthPercentage::length(self.px(vertical)),
            LengthPercentage::length(self.px(horizontal)),
        );
        Rect {
            left: h,
            right: h,
            top: v,
            bottom: v,
        }
    }

    fn gap(&self, points: f32) -> Size<LengthPercentage> {
        Size {
            width: LengthPercentage::length(self.px(points)),
            height: LengthPercentage::length(self.px(points)),
        }
    }
}
