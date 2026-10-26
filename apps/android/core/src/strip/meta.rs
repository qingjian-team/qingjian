//! 画完一帧候选条交给壳的说明（JSON 过 JNI）；像素另经 `copyStrip` 拷进壳的缓冲区。

use serde::Serialize;

use super::StripHit;

#[derive(Debug, Serialize)]
pub struct StripMeta {
    /// 位图像素宽高；像素是预乘 RGBA，与 `Bitmap.Config.ARGB_8888` 的内存布局一致。
    pub width: u32,

    pub height: u32,

    /// 这一行以上是拼音行，壳钉在视口上；以下的候选随横向滚动。
    pub candidates_top: u32,

    /// 主题的底色（ARGB），位图没铺满的地方壳用它填。
    pub background: u32,

    /// 可点区域，译文在前（先命中）。
    pub hits: Vec<StripHit>,

    /// 候选词里有 emoji 的下标：渲染器画不出 COLRv1 的彩色 emoji，壳在词的位置上自己画。
    pub emoji: Vec<usize>,
}
