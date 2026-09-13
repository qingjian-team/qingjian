//! 偏好设置窗口本体：把各页（`pages/`）装进标签视图，底部一行状态；刷新时逐页同步。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSTabView, NSTabViewItem, NSTextField, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use qingjian_core::{Language, UsageSummary, VocabularySummary};
use qingjian_platform::Config;

use super::controls::{language_label, small_label};
use super::layout::{Layout, PAGE_PADDING, PAGE_WIDTH};
use super::pages::{
    AdvancedPage, CandidatesPage, CloudPage, DictionariesPage, FuzzyPage, GeneralPage, PhrasesPage,
    ShortcutsPage, UsagePage, build_about,
};
use super::panel::PreferencesPanel;
use super::target::PreferencesTarget;
use crate::host::DictionaryInfo;

/// 每页顶部留白、页面最低高度（矮页也撑到这个高度，切页时窗口不跳）。
const PAGE_TOP: f64 = 18.0;
const MIN_PAGE_HEIGHT: f64 = 250.0;

/// 标签视图四周留白、底部状态行高度。
const TAB_MARGIN: f64 = 14.0;
const STATUS_HEIGHT: f64 = 18.0;

/// 设置窗口与需要按配置刷新的各页。
pub struct PreferencesWindow {
    /// 窗口。
    panel: Retained<PreferencesPanel>,

    /// 「通用」页。
    general: GeneralPage,

    /// 「候选窗口」页。
    candidates: CandidatesPage,

    /// 「快捷键」页。
    shortcuts: ShortcutsPage,

    /// 自定义文本编辑。
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

/// 一页：标题、布局器、承载视图。
type Page = (&'static str, Layout, Retained<NSView>);

impl PreferencesWindow {
    /// `languages` 是打进包里的释义表语言，`version` / `build` 显示在「关于」页。
    pub fn new(mtm: MainThreadMarker, languages: &[Language], version: &str, build: &str) -> Self {
        let target = PreferencesTarget::new(mtm);
        let new_layout = || Layout::new(PAGE_WIDTH, PAGE_TOP);
        let page = |title: &'static str, layout: Layout| -> Page {
            (
                title,
                layout,
                NSView::initWithFrame(mtm.alloc(), NSRect::ZERO),
            )
        };
        let mut pages: Vec<Page> = Vec::new();

        let mut layout = new_layout();
        let general = GeneralPage::build(&mut layout, mtm, &target, languages);
        pages.push(page("通用", layout));

        let mut layout = new_layout();
        let candidates = CandidatesPage::build(&mut layout, mtm, &target);
        pages.push(page("候选窗口", layout));

        let mut layout = new_layout();
        let shortcuts = ShortcutsPage::build(&mut layout, mtm, &target);
        pages.push(page("快捷键", layout));

        let mut layout = new_layout();
        let phrases = PhrasesPage::build(&mut layout, mtm, &target);
        pages.push(page("快捷文本", layout));

        let mut layout = new_layout();
        let fuzzy = FuzzyPage::build(&mut layout, mtm, &target);
        pages.push(page("模糊音", layout));

        let mut layout = new_layout();
        let dictionaries = DictionariesPage::build(&mut layout, mtm, &target);
        pages.push(page("词库", layout));

        let mut layout = new_layout();
        let cloud = CloudPage::build(&mut layout, mtm, &target);
        pages.push(page("云服务", layout));

        let mut layout = new_layout();
        let advanced = AdvancedPage::build(&mut layout, mtm, &target);
        pages.push(page("高级", layout));

        let mut layout = new_layout();
        let usage = UsagePage::build(&mut layout, mtm);
        pages.push(page("统计", layout));

        let mut layout = new_layout();
        build_about(&mut layout, mtm, &target, version, build);
        pages.push(page("关于", layout));

        // 标签视图：先用临时尺寸量出边框与标签栏占多少，再按最高的一页定最终尺寸
        let page_height = pages
            .iter()
            .map(|(_, layout, _)| layout.height() + PAGE_TOP)
            .fold(MIN_PAGE_HEIGHT, f64::max);
        let probe = NSRect::new(NSPoint::ZERO, NSSize::new(PAGE_WIDTH, page_height));
        let tabs = NSTabView::initWithFrame(mtm.alloc(), probe);
        let inner = tabs.contentRect();
        let chrome_width = PAGE_WIDTH - inner.size.width;
        let chrome_height = page_height - inner.size.height;
        let tabs_size = NSSize::new(PAGE_WIDTH + chrome_width, page_height + chrome_height);
        let content_size = NSSize::new(
            tabs_size.width + 2.0 * TAB_MARGIN,
            tabs_size.height + 2.0 * TAB_MARGIN + STATUS_HEIGHT,
        );
        tabs.setFrame(NSRect::new(
            NSPoint::new(TAB_MARGIN, TAB_MARGIN + STATUS_HEIGHT),
            tabs_size,
        ));
        for (title, layout, view) in pages {
            view.setFrame(NSRect::new(
                NSPoint::ZERO,
                NSSize::new(PAGE_WIDTH, page_height),
            ));
            layout.finish(&view, page_height);
            // SAFETY: identifier 允许为空；条目随 NSTabView 活着
            let item = unsafe { NSTabViewItem::initWithIdentifier(mtm.alloc(), None) };
            item.setLabel(&NSString::from_str(title));
            item.setView(Some(&view));
            tabs.addTabViewItem(&item);
        }
        let content = NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::ZERO, content_size));
        content.addSubview(&tabs);
        let status = small_label(mtm, "");
        status.setTextColor(Some(&NSColor::systemRedColor()));
        status.setFrame(NSRect::new(
            NSPoint::new(TAB_MARGIN + PAGE_PADDING, TAB_MARGIN / 2.0),
            NSSize::new(
                content_size.width - 2.0 * (TAB_MARGIN + PAGE_PADDING),
                STATUS_HEIGHT,
            ),
        ));
        content.addSubview(&status);
        let panel = PreferencesPanel::new(mtm, NSRect::new(NSPoint::ZERO, content_size));
        panel.setTitle(&NSString::from_str("青简偏好设置"));
        panel.setContentView(Some(&content));
        panel.center();

        Self {
            panel,
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
    pub fn phrase_draft(&self) -> (Option<usize>, qingjian_core::CustomPhrase) {
        (self.phrases.selected(), self.phrases.draft())
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
