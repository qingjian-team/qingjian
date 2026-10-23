//! 菜单面板的内容视图：贴位图，跟着鼠标换悬停项重画，点中一项就关面板再交出编号。

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSCompositingOperation, NSEvent, NSImage, NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSPoint, NSRect};
use qingjian_render::{FontLibrary, HitTarget, Rendered, Renderer};

use super::MenuPick;
use crate::candidates::bitmap::{bounds, to_image};
use crate::candidates::bounds::ViewBounds;

#[derive(Default)]
pub struct State {
    /// 第一次弹菜单时才建（加载字体库要几十毫秒）；建不成为 `Some(None)`，不再试。
    renderer: Option<Option<Renderer>>,

    items: Vec<(u32, &'static str)>,

    hovered: Option<usize>,

    dark: bool,

    scale: f32,

    rendered: Option<Rendered>,

    image: Option<Retained<NSImage>>,

    bounds: Option<ViewBounds>,

    pick: Option<MenuPick>,
}

define_class!(
    // SAFETY: NSView 允许子类化；没有实现 Drop。
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = RefCell<State>]
    pub struct MenuView;

    impl MenuView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        /// 面板不激活，第一下点击就要算数。
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            let item = self.item_at(event);
            self.hover(item);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.hover(None);
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.picked(event);
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            self.picked(event);
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let state = self.ivars().borrow();
            let (Some(image), Some(bounds)) = (&state.image, &state.bounds) else {
                return;
            };
            // SAFETY: hints 传 None，其余参数都是普通值；在 drawRect: 内调用，有当前图形上下文。
            unsafe {
                image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                    NSRect::new(NSPoint::ZERO, bounds.size),
                    NSRect::ZERO,
                    NSCompositingOperation::Copy,
                    1.0,
                    true,
                    None,
                );
            }
        }
    }
);

impl MenuView {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm
            .alloc::<Self>()
            .set_ivars(RefCell::new(State::default()));
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] };
        let options = NSTrackingAreaOptions::MouseMoved
            | NSTrackingAreaOptions::MouseEnteredAndExited
            | NSTrackingAreaOptions::ActiveAlways
            | NSTrackingAreaOptions::InVisibleRect;
        // SAFETY: owner 是这个视图，跟踪区域随视图一起释放；userInfo 不用
        let area = unsafe {
            NSTrackingArea::initWithRect_options_owner_userInfo(
                mtm.alloc::<NSTrackingArea>(),
                NSRect::ZERO,
                options,
                Some(&this),
                None,
            )
        };
        this.addTrackingArea(&area);
        this
    }

    /// 换上这份菜单画一遍，返回位图大小与内容区；画不出来返回 `None`。
    pub(super) fn open(
        &self,
        items: Vec<(u32, &'static str)>,
        dark: bool,
        scale: f32,
        pick: MenuPick,
    ) -> Option<ViewBounds> {
        {
            let mut state = self.ivars().borrow_mut();
            state.items = items;
            state.hovered = None;
            state.dark = dark;
            state.scale = scale;
            state.pick = Some(pick);
        }
        self.repaint();
        self.ivars().borrow().bounds
    }

    pub(super) fn close(&self) {
        let mut state = self.ivars().borrow_mut();
        state.pick = None;
        state.hovered = None;
    }

    fn hover(&self, item: Option<usize>) {
        if self.ivars().borrow().hovered == item {
            return;
        }
        self.ivars().borrow_mut().hovered = item;
        self.repaint();
        self.setNeedsDisplay(true);
    }

    fn picked(&self, event: &NSEvent) {
        let Some(item) = self.item_at(event) else {
            return;
        };
        let (pick, id) = {
            let state = self.ivars().borrow();
            (state.pick.clone(), state.items.get(item).map(|(id, _)| *id))
        };
        let (Some(pick), Some(id)) = (pick, id) else {
            return;
        };
        if let Some(window) = self.window() {
            window.orderOut(None);
        }
        self.close();
        pick(id);
    }

    /// 鼠标在第几项上。
    fn item_at(&self, event: &NSEvent) -> Option<usize> {
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        let state = self.ivars().borrow();
        let rendered = state.rendered.as_ref()?;
        let x = point.x as f32 - rendered.content_x as f32 / rendered.scale;
        let y = point.y as f32 - rendered.content_y as f32 / rendered.scale;
        match rendered.hit_at(x, y)? {
            HitTarget::MenuItem(item) => Some(item),
            HitTarget::Candidate(_) | HitTarget::Translation { .. } => None,
        }
    }

    fn repaint(&self) {
        let mut state = self.ivars().borrow_mut();
        let State {
            renderer,
            items,
            hovered,
            dark,
            scale,
            ..
        } = &mut *state;
        let renderer = renderer.get_or_insert_with(|| match FontLibrary::system("zh-CN") {
            Ok(library) => Some(Renderer::new(library)),
            Err(error) => {
                tracing::warn!(%error, "右键菜单的字体库加载失败");
                None
            }
        });
        let Some(renderer) = renderer.as_mut() else {
            return;
        };
        let titles: Vec<&str> = items.iter().map(|(_, title)| *title).collect();
        let rendered = match renderer.render_menu(&titles, *hovered, *dark, *scale) {
            Ok(rendered) => rendered,
            Err(error) => {
                tracing::warn!(%error, "右键菜单渲染失败");
                return;
            }
        };
        let view_bounds = bounds(&rendered);
        state.image = to_image(&rendered.pixmap, view_bounds.size);
        state.bounds = Some(view_bounds);
        state.rendered = Some(rendered);
    }
}
