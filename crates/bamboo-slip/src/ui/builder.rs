//! 一帧的建树入口：包着场景树，管 key 的作用域，收集可交互元素。

use std::hash::Hash;
use std::time::Duration;

use taffy::NodeId;

use super::{ElementState, Elements, Interact, Interaction, Key, ScrollExtent, Searchable};
use crate::animation::Easing;
use crate::scene::Scene;

pub struct Ui<'a, M> {
    scene: Scene,

    interaction: &'a Interaction<M>,

    /// 当前作用域，[`Self::key`] 在它下面派生。
    scope: Key,

    elements: Vec<(Key, NodeId, Interact<M>)>,

    /// 浮在整窗最上面的节点（悬停说明），见 [`Self::overlay`]。
    overlays: Vec<NodeId>,

    /// 这一帧登记的可搜索项，见 [`Self::searchable`]。
    searchables: Vec<Searchable>,

    /// 控件登记、还没被设置行收走的关键词与登记它的节点，见 [`Self::keyword`]。
    keywords: Vec<(NodeId, String)>,

    /// ⓘ 的链接：说明末尾那行提示，与按行标题给出点击消息的函数，见 [`Self::info_links`]。
    info_links: Option<(&'a str, InfoLinks<'a, M>)>,
}

type InfoLinks<'a, M> = Box<dyn Fn(&str) -> Option<M> + 'a>;

impl<'a, M> Ui<'a, M> {
    pub fn new(interaction: &'a Interaction<M>) -> Self {
        Self {
            scene: Scene::new(),
            interaction,
            scope: Key::ROOT,
            elements: Vec::new(),
            overlays: Vec::new(),
            searchables: Vec::new(),
            keywords: Vec::new(),
            info_links: None,
        }
    }

    pub fn scene(&mut self) -> &mut Scene {
        &mut self.scene
    }

    /// 当前作用域下的 key。同一作用域里本地名不能重复，列表项用数据的 id 当本地名，排序变了状态跟着走。
    pub fn key(&self, local: impl Hash) -> Key {
        self.scope.child(local)
    }

    /// 在子作用域里建：组件里的 key 都派生在它下面，同一个组件用几次也不会撞。
    pub fn scope<R>(&mut self, local: impl Hash, build: impl FnOnce(&mut Self) -> R) -> R {
        let outer = self.scope;
        self.scope = outer.child(local);
        let result = build(self);
        self.scope = outer;
        result
    }

    pub fn state(&self, key: Key) -> ElementState {
        self.interaction.state(key)
    }

    /// 元素上一帧在根坐标里的矩形（左、上、宽、高，像素）：弹出层贴着它定位。第一帧还没有。
    pub fn rect(&self, key: Key) -> Option<(f32, f32, f32, f32)> {
        self.interaction.rect(key)
    }

    /// 登记一项可搜索的内容（设置行、分组标题这类控件自己调）。`owned` 是这一项装着的节点（行的控件），
    /// 它们子树里控件登记的关键词归这一项；`keywords` 是使用方另补的同义词。
    pub fn searchable(
        &mut self,
        key: Key,
        (title, detail): (&str, Option<&str>),
        owned: &[NodeId],
        keywords: &[&str],
    ) {
        let mut claimed: Vec<String> = Vec::new();
        let scene = &self.scene;
        self.keywords.retain(|(node, word)| {
            let mine = owned.iter().any(|&root| scene.within(*node, root));
            if mine {
                claimed.push(word.clone());
            }
            !mine
        });
        claimed.extend(keywords.iter().map(|word| (*word).to_owned()));
        self.searchables.push(Searchable {
            key,
            title: title.to_owned(),
            detail: detail.map(str::to_owned),
            keywords: claimed,
        });
    }

    /// 设置行的 ⓘ 可点：`resolve` 按行标题给出点击消息（打开文档这类），给了的 ⓘ 点得动，说明末尾多一行 `hint`。
    pub fn info_links(&mut self, hint: &'a str, resolve: impl Fn(&str) -> Option<M> + 'a) {
        self.info_links = Some((hint, Box::new(resolve)));
    }

    /// 标题为 `title` 的行，ⓘ 的点击消息与提示。
    pub fn info_link(&self, title: &str) -> Option<(M, &'a str)> {
        let (hint, resolve) = self.info_links.as_ref()?;
        resolve(title).map(|message| (message, *hint))
    }

    /// 控件上显示的字（`node` 是控件的根节点），装着它的设置行建好时收作搜索关键词。
    pub fn keyword(&mut self, node: NodeId, text: &str) {
        if !text.is_empty() {
            self.keywords.push((node, text.to_owned()));
        }
    }

    /// 取走这一帧登记的可搜索项：使用方离屏建一遍各页，收集成搜索索引。
    pub fn take_searchables(&mut self) -> Vec<Searchable> {
        std::mem::take(&mut self.searchables)
    }

    /// `key` 是应用要醒目标出的那一项（[`crate::App::spotlight`]）。
    pub fn spotlighted(&self, key: Key) -> bool {
        self.interaction.spotlight() == Some(key)
    }

    /// 输入框光标这一刻亮着，见 [`Interaction::caret_visible`]。
    pub fn caret_visible(&self) -> bool {
        self.interaction.caret_visible()
    }

    /// 滚动区的滚动位置与上一帧的大小，见 [`Interaction::scroll_offset`]、[`Interaction::scroll_extent`]。
    pub fn scroll(&self, key: Key) -> ((f32, f32), Option<ScrollExtent>) {
        (
            self.interaction.scroll_offset(key),
            self.interaction.scroll_extent(key),
        )
    }

    /// 登记 `node` 可交互；矩形取它布局后的盒子。
    pub fn interact(&mut self, node: NodeId, key: Key, interact: Interact<M>) {
        self.elements.push((key, node, interact));
    }

    /// `node` 的位置、大小、不透明度、颜色与上一帧同 key 的节点不同时，从旧值平滑过渡过去。
    /// 交给 `Stage::present` 画才有过渡，`Stage::rasterize` 直接画终点。
    pub fn transition(&mut self, node: NodeId, key: Key, duration: Duration, easing: Easing) {
        self.scene
            .set_transition(node, key.transition_name(), duration, easing);
    }

    /// 同 [`Self::transition`]，但只渐变颜色与不透明度（悬停、按下这类状态）；位置、大小跟着排版直接跳。
    pub fn fade(&mut self, node: NodeId, key: Key, duration: Duration, easing: Easing) {
        self.scene
            .set_fade(node, key.transition_name(), duration, easing);
    }

    /// 子孙的滑动过渡以 `node` 为参照，见 [`Scene::set_anchor`]。控件里会滑的部件（开关的圆钮、分段的底块）
    /// 以控件为参照，控件整个被排版挤动时就不会拖在后面。
    pub fn anchor(&mut self, node: NodeId) {
        self.scene.set_anchor(node);
    }

    /// 浮层：建好后挂到根节点最后，盖在所有东西上面、不被滚动区裁掉。节点要自己绝对定位（根坐标）。
    pub fn overlay(&mut self, node: NodeId) {
        self.overlays.push(node);
    }

    /// 把浮层挂到 `root` 上；[`crate::Runner`] 建完树后调。
    pub fn attach_overlays(&mut self, root: NodeId) -> Result<(), crate::error::RenderError> {
        for node in std::mem::take(&mut self.overlays) {
            self.scene.append(root, node)?;
        }
        Ok(())
    }

    pub fn finish(self) -> (Scene, Elements<M>) {
        (self.scene, Elements(self.elements))
    }
}
