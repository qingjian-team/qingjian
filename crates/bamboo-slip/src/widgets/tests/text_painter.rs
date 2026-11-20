use super::super::*;
use crate::fonts::FontLibrary;
use crate::text::TextPainter;

#[test]
fn fitting_text_cuts_it_and_adds_an_ellipsis() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let style = theme.text(13.0, 400, theme.colors.text);
    let whole = "五笔测试（不自动上屏）";
    let width = text.measure(whole, &style).width;
    assert_eq!(text.fit(whole, &style, width), whole);
    let fitted = text.fit(whole, &style, width / 2.0);
    assert!(
        fitted.ends_with('…') && fitted.len() < whole.len(),
        "{fitted}"
    );
    assert!(text.measure(&fitted, &style).width <= width / 2.0 + 0.5);
    assert_eq!(text.fit(whole, &style, 1.0), "…");
}
