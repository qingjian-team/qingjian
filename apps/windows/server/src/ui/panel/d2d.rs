//! 面板用 Direct2D + DirectWrite 画：GDI 画不出彩色 emoji。画布是分层窗口合成器的那张 DIB，
//! 用 DC 渲染目标绑上去；量字也走 DirectWrite，量的与画的是同一套排版。

use windows::Win32::Foundation::{COLORREF, POINT, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, D2D1_ELLIPSE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
    D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_METRICS, DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER,
    DWRITE_WORD_WRAPPING_NO_WRAP, DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat,
    IDWriteTextLayout,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{GetViewportOrgEx, HDC, SetViewportOrgEx};
use windows::core::{Result, w};
use windows_numerics::Vector2;

/// 一块矩形（物理像素）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub(super) fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub(super) fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// DirectWrite 工厂与 DC 渲染目标，UI 线程上建一次。
pub(super) struct Direct2D {
    write: IDWriteFactory,

    target: ID2D1DCRenderTarget,
}

impl Direct2D {
    pub(super) fn new() -> Result<Self> {
        let factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }?;
        let write: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }?;
        // dpi 96：一个 DIP 就是一个物理像素，尺寸都由我们按 DPI 算好
        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let target = unsafe { factory.CreateDCRenderTarget(&properties) }?;
        Ok(Self { write, target })
    }

    /// 微软雅黑 UI，`px` 是字号（物理像素）；缺字（emoji、生僻符号）由 DirectWrite 回落。
    pub(super) fn format(&self, px: f32) -> Result<IDWriteTextFormat> {
        unsafe {
            self.write.CreateTextFormat(
                w!("Microsoft YaHei UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                px,
                w!("zh-CN"),
            )
        }
    }

    /// 单行排版；`max_width` 之外的截成省略号。
    fn layout(
        &self,
        text: &str,
        format: &IDWriteTextFormat,
        max_width: f32,
    ) -> Result<IDWriteTextLayout> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let layout = unsafe {
            self.write
                .CreateTextLayout(&wide, format, max_width.max(1.0), f32::MAX)
        }?;
        unsafe {
            layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            let sign = self.write.CreateEllipsisTrimmingSign(format)?;
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            layout.SetTrimming(&trimming, &sign)?;
        }
        Ok(layout)
    }

    /// 一行字的宽高。量不出来按零算。
    pub(super) fn measure(&self, text: &str, format: &IDWriteTextFormat) -> (f32, f32) {
        let metrics = self
            .layout(text, format, f32::MAX / 4.0)
            .and_then(|layout| {
                let mut metrics = DWRITE_TEXT_METRICS::default();
                unsafe { layout.GetMetrics(&mut metrics) }?;
                Ok(metrics)
            });
        metrics.map_or((0.0, 0.0), |m| {
            (m.widthIncludingTrailingWhitespace, m.height)
        })
    }

    /// 绑上 `hdc` 里内容区的 `area`（内容坐标）开画；`draw` 里的坐标也是内容坐标。
    /// 合成器把视口原点挪到了内容左上角，D2D 不认视口，先换回设备坐标再绑。
    pub(super) fn paint(
        &self,
        hdc: HDC,
        area: RECT,
        background: COLORREF,
        draw: impl FnOnce(&Canvas),
    ) -> Result<()> {
        let mut origin = POINT::default();
        unsafe {
            let _ = GetViewportOrgEx(hdc, &mut origin);
            let _ = SetViewportOrgEx(hdc, 0, 0, None);
        }
        let bound = RECT {
            left: origin.x + area.left,
            top: origin.y + area.top,
            right: origin.x + area.right,
            bottom: origin.y + area.bottom,
        };
        let result = unsafe { self.target.BindDC(hdc, &bound) }.and_then(|()| {
            let canvas = Canvas {
                d2d: self,
                offset: (area.left as f32, area.top as f32),
            };
            unsafe {
                self.target.BeginDraw();
                self.target
                    .SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
                self.target.Clear(Some(&color(background)));
            }
            draw(&canvas);
            unsafe { self.target.EndDraw(None, None) }
        });
        unsafe {
            let _ = SetViewportOrgEx(hdc, origin.x, origin.y, None);
        }
        result
    }
}

/// 一次绘制里的画笔：坐标是内容坐标，减去绑定区左上角再交给 D2D。画失败的那一笔跳过。
pub(super) struct Canvas<'a> {
    d2d: &'a Direct2D,

    offset: (f32, f32),
}

impl Canvas<'_> {
    pub(super) fn measure(&self, text: &str, format: &IDWriteTextFormat) -> (f32, f32) {
        self.d2d.measure(text, format)
    }

    fn rect(&self, r: Rect) -> D2D_RECT_F {
        D2D_RECT_F {
            left: r.x - self.offset.0,
            top: r.y - self.offset.1,
            right: r.x + r.w - self.offset.0,
            bottom: r.y + r.h - self.offset.1,
        }
    }

    pub(super) fn fill(&self, r: Rect, fill: COLORREF) {
        let target = &self.d2d.target;
        if let Ok(brush) = unsafe { target.CreateSolidColorBrush(&color(fill), None) } {
            unsafe { target.FillRectangle(&self.rect(r), &brush) };
        }
    }

    pub(super) fn fill_round(&self, r: Rect, radius: f32, fill: COLORREF) {
        let target = &self.d2d.target;
        if let Ok(brush) = unsafe { target.CreateSolidColorBrush(&color(fill), None) } {
            let rounded = D2D1_ROUNDED_RECT {
                rect: self.rect(r),
                radiusX: radius,
                radiusY: radius,
            };
            unsafe { target.FillRoundedRectangle(&rounded, &brush) };
        }
    }

    pub(super) fn fill_circle(&self, r: Rect, fill: COLORREF) {
        let target = &self.d2d.target;
        if let Ok(brush) = unsafe { target.CreateSolidColorBrush(&color(fill), None) } {
            let ellipse = D2D1_ELLIPSE {
                point: Vector2 {
                    X: r.x + r.w / 2.0 - self.offset.0,
                    Y: r.y + r.h / 2.0 - self.offset.1,
                },
                radiusX: r.w / 2.0,
                radiusY: r.h / 2.0,
            };
            unsafe { target.FillEllipse(&ellipse, &brush) };
        }
    }

    /// 一行字，左上角在 `(x, y)`，超过 `max_width` 截成省略号；返回画了多宽。
    pub(super) fn text(
        &self,
        text: &str,
        format: &IDWriteTextFormat,
        fill: COLORREF,
        x: f32,
        y: f32,
        max_width: f32,
    ) -> f32 {
        let Ok(layout) = self.d2d.layout(text, format, max_width) else {
            return 0.0;
        };
        self.draw_layout(&layout, fill, x, y);
        let mut metrics = DWRITE_TEXT_METRICS::default();
        if unsafe { layout.GetMetrics(&mut metrics) }.is_err() {
            return 0.0;
        }
        metrics.width.min(max_width)
    }

    /// 一行字居中放进 `r`，放不下截成省略号。
    pub(super) fn text_centered(
        &self,
        text: &str,
        format: &IDWriteTextFormat,
        fill: COLORREF,
        r: Rect,
    ) {
        let Ok(layout) = self.d2d.layout(text, format, r.w) else {
            return;
        };
        unsafe {
            let _ = layout.SetMaxHeight(r.h);
            let _ = layout.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = layout.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
        }
        self.draw_layout(&layout, fill, r.x, r.y);
    }

    fn draw_layout(&self, layout: &IDWriteTextLayout, fill: COLORREF, x: f32, y: f32) {
        let target = &self.d2d.target;
        let Ok(brush) = (unsafe { target.CreateSolidColorBrush(&color(fill), None) }) else {
            return;
        };
        let origin = Vector2 {
            X: x - self.offset.0,
            Y: y - self.offset.1,
        };
        unsafe {
            target.DrawTextLayout(
                origin,
                layout,
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT | D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        }
    }
}

fn color(c: COLORREF) -> D2D1_COLOR_F {
    let channel = |shift: u32| ((c.0 >> shift) & 0xFF) as f32 / 255.0;
    D2D1_COLOR_F {
        r: channel(0),
        g: channel(8),
        b: channel(16),
        a: 1.0,
    }
}

/// 系统强调色（设置 → 个性化 → 颜色），读不到用 Windows 缺省的蓝。注册表里是 0xAABBGGRR，与 COLORREF 同序。
pub(super) fn accent_color() -> COLORREF {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\DWM")
        .and_then(|key| key.get_u32("AccentColor"))
        .map_or(COLORREF(0x00C0_6700), |value| COLORREF(value & 0x00FF_FFFF))
}
