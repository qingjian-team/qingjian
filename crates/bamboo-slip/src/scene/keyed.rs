//! 过渡用的节点登记：带配对名的节点、它们在窗口里的位置，以及动画中盖过布局的位置。

use std::collections::HashMap;
use std::time::Duration;

use taffy::NodeId;

use super::{Scene, Visual};
use crate::animation::{Easing, Placement};
use crate::error::RenderError;

/// 一个带过渡的节点：配对名（同一帧重复的已加序号）、过渡参数、布局给的位置（相对根节点）。
pub struct Keyed {
    pub node: NodeId,

    pub key: String,

    pub duration: Duration,

    pub easing: Easing,

    pub placement: Placement,

    /// 只渐变颜色与不透明度，见 [`Scene::set_fade`]。
    pub fade: bool,

    /// 最近的参照祖先（[`Scene::set_anchor`]）相对根节点的位置；没有参照的为根节点 `(0, 0)`。
    pub anchor: (f32, f32),
}

impl Scene {
    /// 节点要过渡：配对名与参数。
    pub fn set_transition(
        &mut self,
        node: NodeId,
        key: String,
        duration: Duration,
        easing: Easing,
    ) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.transition = Some((key, duration, easing));
        }
    }

    /// 同 [`Self::set_transition`]，但只渐变颜色与不透明度：位置、大小直接到布局给的。
    pub fn set_fade(&mut self, node: NodeId, key: String, duration: Duration, easing: Easing) {
        self.set_transition(node, key, duration, easing);
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.fade = true;
        }
    }

    /// 子孙的滑动过渡按相对 `node` 的位置算：整个 `node` 跟着排版挪动时，里面的东西跟着跳过去，不从旧位置滑。
    pub fn set_anchor(&mut self, node: NodeId) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.anchor = true;
        }
    }

    /// 动画中的位置（画布像素）；`None` 回到布局算的。
    /// 自己不画东西的上级容器拿子节点当阴影形状，它们缓存的遮罩跟着作废；自己有画面的上级不受影响。
    pub fn set_placed(&mut self, node: NodeId, placed: Option<Placement>) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.placed = placed;
        }
        let mut ancestors = Vec::new();
        let mut current = self.tree.parent(node);
        while let Some(parent) = current {
            if matches!(
                self.tree
                    .get_node_context(parent)
                    .map(|context| &context.visual),
                Some(Visual::Group)
            ) {
                ancestors.push(parent);
            }
            current = self.tree.parent(parent);
        }
        self.effect_cache
            .get_mut()
            .retain(|key, _| !ancestors.contains(&key.node()));
    }

    /// 以 `root` 为根、带过渡的节点，按树序；同一帧里同名出现多次的按出现顺序加 `#序号`。要在布局之后调。
    pub fn keyed(&self, root: NodeId) -> Result<Vec<Keyed>, RenderError> {
        let mut found = Vec::new();
        self.collect(root, (0.0, 0.0), (0.0, 0.0), &mut found)?;
        let mut counts: HashMap<String, usize> = HashMap::new();
        for keyed in &found {
            *counts.entry(keyed.key.clone()).or_default() += 1;
        }
        let mut seen: HashMap<String, usize> = HashMap::new();
        for keyed in &mut found {
            if counts[&keyed.key] > 1 {
                let index = seen.entry(keyed.key.clone()).or_default();
                keyed.key = format!("{}#{index}", keyed.key);
                *index += 1;
            }
        }
        Ok(found)
    }

    fn collect(
        &self,
        node: NodeId,
        (x, y): (f32, f32),
        anchor: (f32, f32),
        found: &mut Vec<Keyed>,
    ) -> Result<(), RenderError> {
        let layout = self.tree.layout(node)?;
        let (x, y) = (x + layout.location.x, y + layout.location.y);
        let context = self.tree.get_node_context(node);
        let own = context.map_or(1.0, |context| context.opacity);
        if let Some((key, duration, easing)) =
            context.and_then(|context| context.transition.clone())
        {
            found.push(Keyed {
                node,
                key,
                duration,
                easing,
                placement: Placement {
                    x,
                    y,
                    width: layout.size.width,
                    height: layout.size.height,
                    opacity: own,
                    color: context.and_then(|context| context.visual.color()),
                },
                fade: context.is_some_and(|context| context.fade),
                anchor,
            });
        }
        let anchor = if context.is_some_and(|context| context.anchor) {
            (x, y)
        } else {
            anchor
        };
        for child in self.tree.children(node)? {
            self.collect(child, (x, y), anchor, found)?;
        }
        Ok(())
    }
}
