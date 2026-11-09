//! 设置页的主题预览：用样例帧（敲 nihao 的第一页）按候选条的排法画一张，壳把它贴在主题卡片上、下面自己画几排键。

use std::cell::RefCell;

use qingjian_render::sample::nihao;
use qingjian_render::{FontLibrary, Frame, Layout, Pixmap, Renderer, Tone};

use super::{text_sizes, theme};

/// 样例帧里挑这几个候选：带普通译文、生词译文与不带译文的都有；👋 渲染器画不出（系统 emoji 是 COLRv1），不要。
const SAMPLE_ROWS: [usize; 5] = [0, 3, 2, 4, 5];

/// 与候选条同一个 locale，见 `strip::painter`。
const LOCALE: &str = "zh-CN";

thread_local! {
    /// 设置页在 App 进程里，没有输入法的画笔可借；第一次画时加载字体，之后复用。渲染器不能跨线程，壳固定在一条线程上画预览。
    static RENDERER: RefCell<Option<Renderer>> = const { RefCell::new(None) };
}

/// 主题 `id` 在 `dark` 外观、`text_scale` 字号档下的候选条样子；`scale` 是 dp → 像素的倍数。字体加载失败或画不出时为 `None`。
pub fn preview(id: &str, dark: bool, scale: f32, text_scale: f32) -> Option<Pixmap> {
    RENDERER.with_borrow_mut(|slot| render(slot, id, dark, scale, text_scale))
}

fn render(
    guard: &mut Option<Renderer>,
    id: &str,
    dark: bool,
    scale: f32,
    text_scale: f32,
) -> Option<Pixmap> {
    if guard.is_none() {
        match FontLibrary::system(LOCALE) {
            Ok(library) => *guard = Some(Renderer::new(library)),
            Err(error) => {
                tracing::error!(%error, "预览用的字体加载失败");
                return None;
            }
        }
    }
    let renderer = guard.as_mut()?;
    let theme = theme(id, dark);
    let theme = theme
        .with_text_sizes(text_sizes(&theme, text_scale))
        .with_animations(false);
    match renderer.render(&sample(), Layout::Strip, &theme, scale) {
        Ok(rendered) => Some(rendered.pixmap),
        Err(error) => {
            tracing::warn!(%error, id, "主题预览画不出来");
            None
        }
    }
}

fn sample() -> Frame {
    let mut frame = nihao();
    let rows = std::mem::take(&mut frame.rows);
    frame.rows = SAMPLE_ROWS
        .iter()
        .map(|&from| {
            let mut row = rows[from].clone();
            row.index.clear();
            // 候选条每个词下面只放第一条译文，同 `strip::rows`
            if let Some(cut) = row
                .annotation
                .iter()
                .position(|(_, tone)| *tone == Tone::Separator)
            {
                row.annotation.truncate(cut);
            }
            row
        })
        .collect();
    frame.highlighted = Some(0);
    frame.footer = None;
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_keeps_one_gloss_per_word() {
        let frame = sample();
        assert_eq!(frame.rows.len(), SAMPLE_ROWS.len());
        assert_eq!(frame.rows[0].text, "你好");
        assert!(
            frame.rows[0]
                .annotation
                .iter()
                .all(|(text, _)| text != "hi")
        );
        assert!(frame.rows.iter().all(|row| row.index.is_empty()));
    }
}
