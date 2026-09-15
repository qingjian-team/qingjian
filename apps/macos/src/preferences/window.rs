//! 偏好设置窗口本体：左边侧栏（`sidebar/`）选页，右边翻页器（`pager`）显示各页（`pages/`），
//! 内容列底部一行状态；刷新时逐页同步。

use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSScrollView, NSTextField, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use qingjian_core::{Language, UsageSummary, VocabularySummary};
use qingjian_platform::Config;

use super::controls::{language_label, small_label};
use super::layout::{CARD_MARGIN, Layout, PAGE_WIDTH};
use super::pager::{HEADER_HEIGHT, Pager, PagerPage};
use super::pages::{
    AdvancedPage, CandidatesPage, CloudPage, DictionariesPage, FuzzyPage, GeneralPage, PhrasesPage,
    ShortcutsPage, UsagePage, build_about,
};
use super::panel::PreferencesPanel;
use super::sidebar::{SIDEBAR_WIDTH, Sidebar, SidebarEntry};
use super::target::PreferencesTarget;
use crate::host::DictionaryInfo;

/// 第一张卡片离标题带的距离、最后一张卡片离页底的距离。
const PAGE_TOP: f64 = 4.0;
const PAGE_BOTTOM: f64 = 4.0;

/// 底部状态行的高度与上下留白。
const STATUS_HEIGHT: f64 = 18.0;
const STATUS_MARGIN: f64 = 10.0;

/// 窗口内容的最低高度：侧栏十行加顶部留白要放得下，矮页也不把窗口压扁。
const MIN_CONTENT_HEIGHT: f64 = 400.0;

/// 一页在窗口里最多占多高，再高就装进滚动视图（快捷键页很长，13 寸屏也要放得下整个窗口）。
const MAX_PAGE_HEIGHT: f64 = 620.0;

/// 设置窗口与需要按配置刷新的各页。
pub struct PreferencesWindow {
    /// 窗口。
    panel: Retained<PreferencesPanel>,

    /// 左侧导航栏。
    sidebar: Sidebar,

    /// 右侧翻页器；侧栏的回调也握着一份。
    _pager: Rc<Pager>,

    /// 「通用」页。
    general: GeneralPage,

    /// 「候选窗口」页。
    candidates: CandidatesPage,

    /// 「快捷键」页。
    shortcuts: ShortcutsPage,

    /// 自定义短语编辑。
    phrases: PhrasesPage,

    /// 「模糊音」页。
    fuzzy: FuzzyPage,

    /// 「词库」页。
    dictionaries: DictionariesPage,

    /// 「云服务」页。
    cloud: CloudPage,

    /// 「高级」页。
    advanced: AdvancedPage,

    /// 「统计」页的数字。
    usage: UsagePage,

    /// 底部状态行：配置文件解析失败时显示原因，也给临时提示用。
    status: Retained<NSTextField>,

    /// 所有控件的 target，要和窗口活得一样久。
    _target: Retained<PreferencesTarget>,
}

/// 搭好但还没定高的一页：页名、侧栏图标、布局器。
type Draft = (&'static str, &'static str, Layout);

/// 把过高的页装进滚动视图：透明背景、滚动条自动隐藏，初始停在顶部。
fn scrollable(mtm: MainThreadMarker, page: &NSView, page_height: f64) -> Retained<NSView> {
    let scroll = NSScrollView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::ZERO, NSSize::new(PAGE_WIDTH, MAX_PAGE_HEIGHT)),
    );
    scroll.setDrawsBackground(false);
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setDocumentView(Some(page));
    // 文档视图坐标原点在左下，不滚的话初始露出的是页尾
    let clip = scroll.contentView();
    clip.scrollToPoint(NSPoint::new(0.0, page_height - MAX_PAGE_HEIGHT));
    scroll.reflectScrolledClipView(&clip);
    Retained::into_super(scroll)
}

impl PreferencesWindow {
    /// `languages` 是打进包里的释义表语言，`version` / `build` 显示在「关于」页。
    pub fn new(mtm: MainThreadMarker, languages: &[Language], version: &str, build: &str) -> Self {
        let target = PreferencesTarget::new(mtm);
        let new_layout = || Layout::new(PAGE_WIDTH, PAGE_TOP);
        let mut drafts: Vec<Draft> = Vec::new();

        let mut layout = new_layout();
        let general = GeneralPage::build(&mut layout, mtm, &target, languages);
        drafts.push(("通用", "gearshape", layout));

        let mut layout = new_layout();
        let candidates = CandidatesPage::build(&mut layout, mtm, &target);
        drafts.push(("候选窗口", "macwindow", layout));

        let mut layout = new_layout();
        let shortcuts = ShortcutsPage::build(&mut layout, mtm, &target);
        drafts.push(("快捷键", "keyboard", layout));

        let mut layout = new_layout();
        let phrases = PhrasesPage::build(&mut layout, mtm, &target);
        drafts.push(("自定义短语", "text.quote", layout));

        let mut layout = new_layout();
        let fuzzy = FuzzyPage::build(&mut layout, mtm, &target);
        drafts.push(("模糊音", "waveform", layout));

        let mut layout = new_layout();
        let dictionaries = DictionariesPage::build(&mut layout, mtm, &target);
        drafts.push(("词库", "books.vertical", layout));

        let mut layout = new_layout();
        let cloud = CloudPage::build(&mut layout, mtm, &target);
        drafts.push(("云服务", "cloud", layout));

        let mut layout = new_layout();
        let advanced = AdvancedPage::build(&mut layout, mtm, &target);
        drafts.push(("高级", "slider.horizontal.3", layout));

        let mut layout = new_layout();
        let usage = UsagePage::build(&mut layout, mtm);
        drafts.push(("统计", "chart.bar", layout));

        let mut layout = new_layout();
        build_about(&mut layout, mtm, &target, version, build);
        drafts.push(("关于", "info.circle", layout));

        // 每页按自己的内容定高；切页时窗口跟着伸缩，不再按最高的一页统一撑开
        let mut entries = Vec::with_capacity(drafts.len());
        let mut pages = Vec::with_capacity(drafts.len());
        for (title, symbol, layout) in drafts {
            let height = layout.height() + PAGE_BOTTOM;
            let view = NSView::initWithFrame(
                mtm.alloc(),
                NSRect::new(NSPoint::ZERO, NSSize::new(PAGE_WIDTH, height)),
            );
            layout.finish(&view, height);
            entries.push(SidebarEntry { title, symbol });
            let (view, height) = if height > MAX_PAGE_HEIGHT {
                (scrollable(mtm, &view, height), MAX_PAGE_HEIGHT)
            } else {
                (view, height)
            };
            pages.push(PagerPage {
                title,
                view,
                height,
            });
        }

        let content_size = NSSize::new(SIDEBAR_WIDTH + PAGE_WIDTH, MIN_CONTENT_HEIGHT);
        let panel = PreferencesPanel::new(mtm, NSRect::new(NSPoint::ZERO, content_size));
        panel.setTitle(&NSString::from_str("青简偏好设置"));
        let content = NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::ZERO, content_size));
        let fixed_height = HEADER_HEIGHT + STATUS_HEIGHT + 2.0 * STATUS_MARGIN;
        let pager = Rc::new(Pager::new(
            Retained::into_super(panel.clone()),
            &content,
            pages,
            SIDEBAR_WIDTH,
            SIDEBAR_WIDTH + CARD_MARGIN,
            fixed_height,
            MIN_CONTENT_HEIGHT,
        ));
        let switch = Rc::clone(&pager);
        let sidebar = Sidebar::new(
            mtm,
            entries,
            MIN_CONTENT_HEIGHT,
            Box::new(move |index| switch.show(index)),
        );
        content.addSubview(sidebar.view());
        let status = small_label(mtm, "");
        status.setTextColor(Some(&NSColor::systemRedColor()));
        status.setFrame(NSRect::new(
            NSPoint::new(SIDEBAR_WIDTH + CARD_MARGIN, STATUS_MARGIN),
            NSSize::new(PAGE_WIDTH - 2.0 * CARD_MARGIN, STATUS_HEIGHT),
        ));
        content.addSubview(&status);
        panel.setContentView(Some(&content));
        sidebar.select(0);
        panel.center();

        Self {
            panel,
            sidebar,
            _pager: pager,
            general,
            candidates,
            shortcuts,
            phrases,
            fuzzy,
            dictionaries,
            cloud,
            advanced,
            usage,
            status,
            _target: target,
        }
    }

    /// 切到第 `index` 页（侧栏选中，翻页器跟着切）。
    pub fn select_page(&self, index: usize) {
        self.sidebar.select(index);
    }

    pub fn select_phrase(&self, config: &Config, index: usize) {
        self.phrases.load(config, index);
    }
    pub fn selected_phrase(&self) -> Option<usize> {
        self.phrases.selected_row()
    }
    pub fn edit_phrase(&self, config: &Config, index: Option<usize>) {
        self.phrases.edit(config, index);
    }
    pub fn close_phrase_editor(&self) {
        self.phrases.close_editor();
    }
    pub fn set_phrase_error(&self, error: &str) {
        self.phrases.set_error(error);
    }
    pub fn phrase_draft(
        &self,
        config: &Config,
    ) -> Result<(Option<usize>, qingjian_core::CustomPhrase), String> {
        Ok((self.phrases.selected(config)?, self.phrases.draft()))
    }

    /// 打开（或带到最前）。
    pub fn show(&self) {
        self.panel.present();
    }

    /// 按配置刷新所有控件。`key_present` 是密钥已经有了（环境或配置里）；密钥框永远不回显值。
    pub fn sync(
        &self,
        config: &Config,
        key_present: bool,
        error: Option<&str>,
        dictionaries: &[DictionaryInfo],
    ) {
        self.dictionaries.rebuild(dictionaries);
        self.general.sync(config);
        self.candidates.sync(config);
        self.shortcuts.sync(config);
        self.phrases.sync(config);
        self.fuzzy.sync(config);
        self.cloud.sync(
            config,
            key_present,
            crate::app::paths::model_path().is_some(),
        );
        self.advanced.sync(config);
        let status = error
            .map(|e| format!("配置文件有错误，已沿用上一份：{e}"))
            .unwrap_or_default();
        self.status.setTextColor(Some(&NSColor::systemRedColor()));
        self.status.setStringValue(&NSString::from_str(&status));
    }

    /// 刷新「统计」页。打开窗口时调（数字随时在变，不跟配置一起同步）。
    pub fn sync_usage(
        &self,
        summary: &UsageSummary,
        vocabulary: &VocabularySummary,
        language: Language,
    ) {
        self.usage
            .show(summary, vocabulary, language_label(language));
    }

    /// 底部状态行临时显示一句提示（不是错误，灰字）；下次 `sync` 会被配置状态覆盖。
    pub fn set_status(&self, text: &str) {
        self.status
            .setTextColor(Some(&NSColor::secondaryLabelColor()));
        self.status.setStringValue(&NSString::from_str(text));
    }
}
