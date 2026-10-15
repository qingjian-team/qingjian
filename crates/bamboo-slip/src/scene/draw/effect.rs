//! 画投影与内阴影：把节点的形状画进一张离屏图取 alpha，偏移、模糊、染色后合成。
//! 离屏图只取形状加模糊铺开的那一块，不是整张画布。形状怎么画由调用方给（节点自己的画面，或容器的子节点）。
//! 算好的遮罩交回调用方合成并缓存：动画帧里没动的节点直接贴缓存，不再重画形状、重做模糊。

use crate::canvas::Canvas;
use crate::error::RenderError;
use crate::scene::{Effect, EffectKind};
use crate::shadow;

/// 在离屏图上画形状：盒子在 `rect`（离屏图坐标），按 `spread` 外扩（负数内缩）。
pub(in crate::scene) type DrawShape<'a> =
    dyn FnMut(&mut Canvas, (f32, f32, f32, f32), f32) -> Result<(), RenderError> + 'a;

/// 一个效果算好的遮罩：左上角在画布里的位置、宽高、alpha。
#[derive(Debug, Clone)]
pub(in crate::scene) struct EffectMask {
    pub(in crate::scene) left: i32,

    pub(in crate::scene) top: i32,

    pub(in crate::scene) width: u32,

    pub(in crate::scene) height: u32,

    pub(in crate::scene) alpha: Vec<u8>,
}

/// 算一个效果的遮罩；`rect` 是节点盒子（像素，画布坐标），`bounds` 是画布宽高。整块落在画布外时为 `None`。
pub(in crate::scene) fn effect_mask(
    bounds: (u32, u32),
    rect: (f32, f32, f32, f32),
    effect: &Effect,
    shape: &mut DrawShape,
) -> Result<Option<EffectMask>, RenderError> {
    let (x, y, width, height) = rect;
    let pad = shadow::reach(effect.blur) + effect.spread.abs();
    let (sx, sy) = match effect.kind {
        EffectKind::DropShadow => (x + effect.x, y + effect.y),
        EffectKind::InnerShadow => (x, y),
    };
    // 离屏区域：形状（投影按偏移挪过）四周加模糊铺开的宽度，夹在画布里
    let left = ((sx - pad).floor() as i32).max(0);
    let top = ((sy - pad).floor() as i32).max(0);
    let right = ((sx + width + pad).ceil() as i32).min(bounds.0 as i32);
    let bottom = ((sy + height + pad).ceil() as i32).min(bounds.1 as i32);
    if right <= left || bottom <= top {
        return Ok(None);
    }
    let (w, h) = ((right - left) as u32, (bottom - top) as u32);
    let (ox, oy) = (left as f32, top as f32);
    let shifted = (x + effect.x - ox, y + effect.y - oy, width, height);
    let mask = match effect.kind {
        EffectKind::DropShadow => {
            let mut mask = alpha(shape, shifted, effect.spread, (w, h))?;
            shadow::blur(&mut mask, w as usize, h as usize, effect.blur);
            mask
        }
        EffectKind::InnerShadow => {
            let inside = alpha(shape, (x - ox, y - oy, width, height), 0.0, (w, h))?;
            // 形状外（按偏移挪过、按扩展缩过）为实，模糊后只留形状里的部分
            let mut mask = alpha(shape, shifted, -effect.spread, (w, h))?;
            for value in &mut mask {
                *value = 255 - *value;
            }
            shadow::blur(&mut mask, w as usize, h as usize, effect.blur);
            for (value, inside) in mask.iter_mut().zip(&inside) {
                *value = ((u32::from(*value) * u32::from(*inside) + 127) / 255) as u8;
            }
            mask
        }
    };
    Ok(Some(EffectMask {
        left,
        top,
        width: w,
        height: h,
        alpha: mask,
    }))
}

/// 形状画进一张 `size` 大的离屏图，取 alpha。
fn alpha(
    shape: &mut DrawShape,
    rect: (f32, f32, f32, f32),
    spread: f32,
    size: (u32, u32),
) -> Result<Vec<u8>, RenderError> {
    let mut layer = Canvas::new(size.0, size.1)?;
    shape(&mut layer, rect, spread)?;
    Ok(layer
        .into_pixmap()
        .pixels()
        .iter()
        .map(|pixel| pixel.alpha())
        .collect())
}
