//! 侧栏的线性图标：16 单位见方的网格上画，1.5 单位线宽、圆头圆角，按 `size` 缩放。

use bamboo_slip::{Canvas, Color, Icon};

use super::pen::Pen;

#[derive(Debug, Clone, Copy)]
pub enum Glyph {
    Keyboard,
    Candidates,
    Language,
    Appearance,
    Command,
    Phrases,
    Dictionary,
    Cloud,
    Stats,
    Vocabulary,
    Advanced,
    Experimental,
    About,
    Search,
}

impl Icon for Glyph {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let mut pen = Pen {
            x,
            y,
            unit: size / 16.0,
            strokes: Vec::new(),
            fills: Vec::new(),
        };
        match self {
            // 候选窗：窗框里一条高亮行
            Glyph::Candidates => {
                pen.round_rect(1.75, 2.75, 12.5, 10.5, 2.0, false);
                pen.round_rect(4.0, 6.0, 5.5, 2.5, 1.0, true);
                pen.line(&[(4.0, 10.75), (11.5, 10.75)]);
            }
            // 键盘
            Glyph::Keyboard => {
                pen.round_rect(1.25, 3.75, 13.5, 8.5, 1.75, false);
                for column in [4.0, 6.67, 9.33, 12.0] {
                    pen.circle(column, 6.5, 0.7, true);
                }
                pen.line(&[(5.0, 9.5), (11.0, 9.5)]);
            }
            // 对话框里两行字
            Glyph::Phrases => {
                pen.line(&[
                    (4.0, 13.5),
                    (4.0, 11.5),
                    (3.0, 11.5),
                    (1.75, 10.25),
                    (1.75, 3.75),
                    (3.0, 2.5),
                    (13.0, 2.5),
                    (14.25, 3.75),
                    (14.25, 10.25),
                    (13.0, 11.5),
                    (7.0, 11.5),
                    (4.0, 13.5),
                ]);
                pen.line(&[(4.75, 5.75), (11.25, 5.75)]);
                pen.line(&[(4.75, 8.25), (9.0, 8.25)]);
            }
            // 翻开的书
            Glyph::Dictionary => {
                pen.line(&[(8.0, 4.0), (8.0, 13.5)]);
                pen.line(&[
                    (8.0, 4.0),
                    (5.5, 2.75),
                    (1.75, 2.75),
                    (1.75, 12.25),
                    (5.5, 12.25),
                    (8.0, 13.5),
                ]);
                pen.line(&[
                    (8.0, 4.0),
                    (10.5, 2.75),
                    (14.25, 2.75),
                    (14.25, 12.25),
                    (10.5, 12.25),
                    (8.0, 13.5),
                ]);
            }
            // 左右两支箭头：中英来回切
            Glyph::Language => {
                pen.line(&[(2.0, 5.25), (13.5, 5.25)]);
                pen.line(&[(10.75, 2.5), (13.5, 5.25), (10.75, 8.0)]);
                pen.line(&[(14.0, 10.75), (2.5, 10.75)]);
                pen.line(&[(5.25, 8.0), (2.5, 10.75), (5.25, 13.5)]);
            }
            // 半边涂满的圆：深浅色
            Glyph::Appearance => {
                pen.circle(8.0, 8.0, 6.5, false);
                let k = 6.5 * 0.552;
                pen.fill_curve(
                    (8.0, 1.5),
                    &[
                        [(8.0 + k, 1.5), (14.5, 8.0 - k), (14.5, 8.0)],
                        [(14.5, 8.0 + k), (8.0 + k, 14.5), (8.0, 14.5)],
                    ],
                );
            }
            // ⌘
            Glyph::Command => {
                for (x, y) in [(4.25, 4.25), (11.75, 4.25), (4.25, 11.75), (11.75, 11.75)] {
                    pen.circle(x, y, 2.0, false);
                }
                pen.line(&[(6.25, 4.25), (6.25, 11.75)]);
                pen.line(&[(9.75, 4.25), (9.75, 11.75)]);
                pen.line(&[(4.25, 6.25), (11.75, 6.25)]);
                pen.line(&[(4.25, 9.75), (11.75, 9.75)]);
            }
            // 云
            Glyph::Cloud => {
                pen.curve(
                    (4.5, 12.75),
                    &[
                        [(2.25, 12.75), (1.0, 11.25), (1.25, 9.5)],
                        [(1.5, 7.75), (3.0, 6.75), (4.5, 7.0)],
                        [(5.0, 4.0), (7.5, 2.75), (10.0, 3.5)],
                        [(12.25, 4.25), (13.25, 6.25), (12.75, 8.0)],
                        [(14.75, 8.5), (15.25, 10.5), (14.25, 11.75)],
                        [(13.5, 12.5), (12.75, 12.75), (11.75, 12.75)],
                    ],
                );
                pen.line(&[(11.75, 12.75), (4.5, 12.75)]);
            }
            // 三根柱
            Glyph::Stats => {
                pen.line(&[(3.5, 13.0), (3.5, 9.0)]);
                pen.line(&[(8.0, 13.0), (8.0, 3.5)]);
                pen.line(&[(12.5, 13.0), (12.5, 6.5)]);
            }
            // 卡片上一个 A，右边两行释义
            Glyph::Vocabulary => {
                pen.round_rect(1.25, 2.25, 13.5, 11.5, 2.0, false);
                pen.line(&[(3.75, 11.0), (5.75, 5.0), (7.75, 11.0)]);
                pen.line(&[(4.5, 8.75), (7.0, 8.75)]);
                pen.line(&[(9.75, 6.25), (12.25, 6.25)]);
                pen.line(&[(9.75, 9.25), (12.25, 9.25)]);
            }
            // 终端提示符
            Glyph::Advanced => {
                pen.round_rect(1.25, 2.25, 13.5, 11.5, 2.0, false);
                pen.line(&[(4.25, 6.0), (6.75, 8.0), (4.25, 10.0)]);
                pen.line(&[(8.5, 10.0), (11.75, 10.0)]);
            }
            // 烧瓶，瓶里一道液面
            Glyph::Experimental => {
                pen.line(&[(5.75, 1.75), (10.25, 1.75)]);
                pen.line(&[
                    (6.75, 1.75),
                    (6.75, 6.0),
                    (2.5, 12.5),
                    (3.5, 14.25),
                    (12.5, 14.25),
                    (13.5, 12.5),
                    (9.25, 6.0),
                    (9.25, 1.75),
                ]);
                pen.line(&[(4.5, 10.25), (11.5, 10.25)]);
            }
            // 圆圈里一个 i
            Glyph::About => {
                pen.circle(8.0, 8.0, 6.5, false);
                pen.circle(8.0, 5.0, 0.9, true);
                pen.line(&[(8.0, 7.5), (8.0, 11.25)]);
            }
            Glyph::Search => {
                pen.circle(7.0, 7.0, 4.75, false);
                pen.line(&[(10.5, 10.5), (14.0, 14.0)]);
            }
        }
        let width = 1.5 * pen.unit;
        for path in &pen.strokes {
            canvas.stroke_outline(path, width, color);
        }
        for path in &pen.fills {
            canvas.fill_path(path, color, bamboo_slip::tiny_skia::BlendMode::SourceOver);
        }
    }
}
