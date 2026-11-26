//! 设置页的主题预览：同一份样例帧按触屏候选条或桌面候选窗布局渲染，壳只负责贴图。

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

/// 主题 `id` 在 `dark` 外观、`text_scale` 字号档下的候选样子；`physical` 选择桌面布局，`scale` 是 dp → 像素的倍数。
pub fn preview(
    id: &str,
    dark: bool,
    scale: f32,
    text_scale: f32,
    physical: bool,
) -> Option<Pixmap> {
    RENDERER.with_borrow_mut(|slot| render(slot, id, dark, scale, text_scale, physical))
}

fn render(
    guard: &mut Option<Renderer>,
    id: &str,
    dark: bool,
    scale: f32,
    text_scale: f32,
    physical: bool,
) -> Option<Pixmap> {
    if guard.is_none() {
        match FontLibrary::system(LOCALE) {
            Ok(library) => *guard = Some(Renderer::new(library)),
            Err(error) => {
                tracing::error!(%error, "Failed to load theme preview fonts");
                return None;
            }
        }
    }
    let renderer = guard.as_mut()?;
    let theme = theme(id, dark);
    let theme = theme
        .with_text_sizes(text_sizes(&theme, text_scale))
        .with_animations(false);
    let layout = if physical {
        Layout::Vertical
    } else {
        Layout::Strip
    };
    match renderer.render(&sample(physical), layout, &theme, scale) {
        Ok(rendered) => Some(rendered.pixmap),
        Err(error) => {
            tracing::warn!(%error, id, "Failed to render theme preview");
            None
        }
    }
}

fn sample(physical: bool) -> Frame {
    let mut frame = nihao();
    let rows = std::mem::take(&mut frame.rows);
    frame.rows = SAMPLE_ROWS
        .iter()
        .enumerate()
        .map(|(index, &from)| {
            let mut row = rows[from].clone();
            row.index = if physical {
                (index + 1).to_string()
            } else {
                String::new()
            };
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
    frame.footer = physical.then(|| "1/2".to_owned());
    frame.next_page = physical;
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_keeps_one_gloss_per_word() {
        let frame = sample(false);
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
    #[test]
    fn physical_sample_has_desktop_numbers_and_paging() {
        let frame = sample(true);
        assert_eq!(
            frame
                .rows
                .iter()
                .map(|r| r.index.as_str())
                .collect::<Vec<_>>(),
            ["1", "2", "3", "4", "5"]
        );
        assert_eq!(frame.footer.as_deref(), Some("1/2"));
        assert!(frame.next_page);
    }
}
