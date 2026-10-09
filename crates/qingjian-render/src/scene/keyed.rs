//! 过渡用的节点登记：带配对名的节点、它们在窗口里的位置，以及动画中盖过布局的位置。

use std::collections::HashMap;
use std::time::Duration;

use taffy::NodeId;

use super::{Scene, Visual};
use crate::animation::{Easing, Placement};
use crate::error::RenderError;

/// 一个带过渡的节点：配对名（同一帧重复的已加序号）、过渡参数、布局给的位置（相对根节点）。
pub(crate) struct Keyed {
    pub(crate) node: NodeId,

    pub(crate) key: String,

    pub(crate) duration: Duration,

    pub(crate) easing: Easing,

    pub(crate) placement: Placement,
}

impl Scene {
    /// 节点要过渡：配对名与参数。
    pub(crate) fn set_transition(
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

    /// 动画中的位置（画布像素）；`None` 回到布局算的。
    /// 自己不画东西的上级容器拿子节点当阴影形状，它们缓存的遮罩跟着作废；自己有画面的上级不受影响。
    pub(crate) fn set_placed(&mut self, node: NodeId, placed: Option<Placement>) {
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
    pub(crate) fn keyed(&self, root: NodeId) -> Result<Vec<Keyed>, RenderError> {
        let mut found = Vec::new();
        self.collect(root, 0.0, 0.0, &mut found)?;
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
        x: f32,
        y: f32,
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
                },
            });
        }
        for child in self.tree.children(node)? {
            self.collect(child, x, y, found)?;
        }
        Ok(())
    }
}
